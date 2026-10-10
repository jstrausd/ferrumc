//! Bounded JSON syntax admission retaining the original decimal number token.
//!
//! FLOAT and DOUBLE codecs round separately from the same token. Materializing
//! an f64 first can double-round a valid FLOAT; it is not an acceptable parser
//! shortcut. Only string escape decoding is delegated to `serde_json`.

use std::collections::BTreeMap;
use std::ops::Range;

use ferrumc_registry::worldgen::{
    MAX_DEFINITION_BYTES, MAX_DEFINITION_CONTAINER_ENTRIES, MAX_DEFINITION_DEPTH,
    MAX_DEFINITION_NODES, MAX_DEFINITION_STRING_BYTES,
};

use super::{DensityCompileError, DensityCompileErrorKind as Kind, DensityLimit};

#[derive(Debug)]
pub(super) struct Value<'a> {
    pub kind: ValueKind<'a>,
    pub span: Range<usize>,
}
#[derive(Debug)]
pub(super) enum ValueKind<'a> {
    Null,
    Bool(bool),
    Number(&'a str),
    String(String),
    Array(Vec<Value<'a>>),
    Object(BTreeMap<String, Value<'a>>),
}
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct Stats {
    pub nodes: usize,
    pub depth: usize,
}

pub(super) fn parse<'a>(
    input: &'a str,
    resource: &'static str,
) -> Result<(Value<'a>, Stats), DensityCompileError> {
    let mut parser = Parser {
        input,
        position: 0,
        resource,
        stats: Stats::default(),
    };
    parser.check(
        input.len(),
        MAX_DEFINITION_BYTES,
        DensityLimit::DefinitionBytes,
    )?;
    let value = parser.value(0)?;
    parser.whitespace();
    if parser.position != input.len() {
        return Err(parser.error(Kind::InvalidJson));
    }
    Ok((value, parser.stats))
}
struct Parser<'a> {
    input: &'a str,
    position: usize,
    resource: &'static str,
    stats: Stats,
}
impl<'a> Parser<'a> {
    fn error(&self, kind: Kind) -> DensityCompileError {
        DensityCompileError {
            resource: self.resource,
            field: "$",
            kind,
            offset: Some(self.position),
        }
    }
    fn check(
        &self,
        count: usize,
        maximum: usize,
        limit: DensityLimit,
    ) -> Result<(), DensityCompileError> {
        if count > maximum {
            return Err(self.error(Kind::Limit {
                limit,
                count,
                maximum,
            }));
        }
        Ok(())
    }
    fn charge(&mut self, depth: usize) -> Result<(), DensityCompileError> {
        self.check(depth, MAX_DEFINITION_DEPTH, DensityLimit::JsonDepth)?;
        self.check(
            self.stats.nodes + 1,
            MAX_DEFINITION_NODES,
            DensityLimit::JsonNodes,
        )?;
        self.stats.nodes += 1;
        self.stats.depth = self.stats.depth.max(depth);
        Ok(())
    }
    fn byte(&self) -> Option<u8> {
        self.input.as_bytes().get(self.position).copied()
    }
    fn whitespace(&mut self) {
        while matches!(self.byte(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.position += 1;
        }
    }
    fn eat(&mut self, byte: u8) -> bool {
        if self.byte() == Some(byte) {
            self.position += 1;
            true
        } else {
            false
        }
    }
    fn literal(&mut self, expected: &str) -> Result<(), DensityCompileError> {
        if self.input[self.position..].starts_with(expected) {
            self.position += expected.len();
            Ok(())
        } else {
            Err(self.error(Kind::InvalidJson))
        }
    }
    fn string(&mut self) -> Result<String, DensityCompileError> {
        let start = self.position;
        if !self.eat(b'"') {
            return Err(self.error(Kind::InvalidJson));
        }
        let mut escaped = false;
        loop {
            let Some(byte) = self.byte() else {
                return Err(self.error(Kind::InvalidJson));
            };
            self.position += 1;
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                break;
            }
        }
        // The complete source was byte-admitted before this bounded decoder can
        // allocate temporary escaped text. The decoded-string cap is separate.
        let string: String = serde_json::from_str(&self.input[start..self.position])
            .map_err(|_| self.error(Kind::InvalidJson))?;
        self.check(
            string.len(),
            MAX_DEFINITION_STRING_BYTES,
            DensityLimit::StringBytes,
        )?;
        Ok(string)
    }
    fn number(&mut self) -> Result<&'a str, DensityCompileError> {
        let start = self.position;
        self.eat(b'-');
        match self.byte() {
            Some(b'0') => {
                self.position += 1;
            }
            Some(b'1'..=b'9') => {
                self.position += 1;
                while matches!(self.byte(), Some(b'0'..=b'9')) {
                    self.position += 1;
                }
            }
            _ => return Err(self.error(Kind::InvalidJson)),
        }
        if self.eat(b'.') {
            let before = self.position;
            while matches!(self.byte(), Some(b'0'..=b'9')) {
                self.position += 1;
            }
            if before == self.position {
                return Err(self.error(Kind::InvalidJson));
            }
        }
        if matches!(self.byte(), Some(b'e' | b'E')) {
            self.position += 1;
            if matches!(self.byte(), Some(b'+' | b'-')) {
                self.position += 1;
            }
            let before = self.position;
            while matches!(self.byte(), Some(b'0'..=b'9')) {
                self.position += 1;
            }
            if before == self.position {
                return Err(self.error(Kind::InvalidJson));
            }
        }
        let token = &self.input[start..self.position];
        Ok(token)
    }
    fn value(&mut self, depth: usize) -> Result<Value<'a>, DensityCompileError> {
        self.whitespace();
        self.charge(depth)?;
        let start = self.position;
        let kind = match self.byte() {
            Some(b'n') => {
                self.literal("null")?;
                ValueKind::Null
            }
            Some(b't') => {
                self.literal("true")?;
                ValueKind::Bool(true)
            }
            Some(b'f') => {
                self.literal("false")?;
                ValueKind::Bool(false)
            }
            Some(b'"') => ValueKind::String(self.string()?),
            Some(b'-' | b'0'..=b'9') => ValueKind::Number(self.number()?),
            Some(b'[') => ValueKind::Array(self.array(depth)?),
            Some(b'{') => ValueKind::Object(self.object(depth)?),
            _ => return Err(self.error(Kind::InvalidJson)),
        };
        Ok(Value {
            kind,
            span: start..self.position,
        })
    }
    fn array(&mut self, depth: usize) -> Result<Vec<Value<'a>>, DensityCompileError> {
        self.position += 1;
        self.whitespace();
        let mut values = Vec::new();
        if self.eat(b']') {
            return Ok(values);
        }
        loop {
            self.check(
                values.len() + 1,
                MAX_DEFINITION_CONTAINER_ENTRIES,
                DensityLimit::ContainerEntries,
            )?;
            let child = self.value(depth + 1)?;
            values.push(child);
            self.whitespace();
            if self.eat(b']') {
                break;
            }
            if !self.eat(b',') {
                return Err(self.error(Kind::InvalidJson));
            }
        }
        Ok(values)
    }
    fn object(&mut self, depth: usize) -> Result<BTreeMap<String, Value<'a>>, DensityCompileError> {
        self.position += 1;
        self.whitespace();
        let mut values = BTreeMap::new();
        if self.eat(b'}') {
            return Ok(values);
        }
        loop {
            self.check(
                values.len() + 1,
                MAX_DEFINITION_CONTAINER_ENTRIES,
                DensityLimit::ContainerEntries,
            )?;
            self.whitespace();
            self.charge(depth + 1)?;
            let key = self.string()?;
            if values.contains_key(&key) {
                return Err(self.error(Kind::DuplicateField));
            }
            self.whitespace();
            if !self.eat(b':') {
                return Err(self.error(Kind::InvalidJson));
            }
            let child = self.value(depth + 1)?;
            values.insert(key, child);
            self.whitespace();
            if self.eat(b'}') {
                break;
            }
            if !self.eat(b',') {
                return Err(self.error(Kind::InvalidJson));
            }
        }
        Ok(values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn grammar_duplicates_and_every_admission_cap() {
        for source in [
            "",
            "01",
            "-",
            "1.",
            "1e+",
            "1 2",
            "[1,]",
            "{\"a\":0,}",
            "{\"a\":1,\"\\u0061\":2}",
            "\"\\ud800\"",
            "\"\\x00\"",
            "NaN",
        ] {
            assert!(parse(source, "test").is_err(), "{source}");
        }
        let (_, stats) = parse("{\"a\":[0,1]}", "test").unwrap();
        assert_eq!((stats.nodes, stats.depth), (5, 2));
        assert!(parse(&format!("{}0{}", "[".repeat(33), "]".repeat(33)), "test").is_err());
        assert!(parse(&serde_json::to_string(&vec![0; 257]).unwrap(), "test").is_err());
        assert!(parse(&serde_json::to_string(&"x".repeat(257)).unwrap(), "test").is_err());
        assert!(parse(&" ".repeat(MAX_DEFINITION_BYTES + 1), "test").is_err());
        assert!(parse(
            &serde_json::to_string(&vec![vec![0; 16]; 256]).unwrap(),
            "test"
        )
        .is_err());
        assert!(parse("1e1000", "test").is_ok());
    }
    #[test]
    fn decimal_tokens_preserve_signed_zero_and_direct_float_rounding() {
        let (value, _) = parse(
            "[-0,-0.0,1.00000005960464477539062500000000000000000000000001]",
            "test",
        )
        .unwrap();
        let ValueKind::Array(values) = value.kind else {
            panic!()
        };
        for value in &values[..2] {
            let ValueKind::Number(token) = value.kind else {
                panic!()
            };
            assert_eq!(token.parse::<f64>().unwrap().to_bits(), (-0.0f64).to_bits());
        }
        let ValueKind::Number(token) = values[2].kind else {
            panic!()
        };
        assert_ne!(
            token.parse::<f32>().unwrap().to_bits(),
            (token.parse::<f64>().unwrap() as f32).to_bits()
        );
    }

    #[test]
    fn unicode_escape_grammar_and_decoded_duplicate_keys() {
        for source in [r#""\ud83d\ude00""#, r#""Grüße🌍""#, r#""\"\\\/\b\f\n\r\t""#] {
            assert!(parse(source, "test").is_ok(), "{source}");
        }
        for source in [
            r#""\ud800""#,
            r#""\udc00""#,
            r#""\ud800\u0000""#,
            r#""\u000g""#,
            r#""\u123""#,
            r#"{"é":0,"\u00e9":1}"#,
            "\"raw\nnewline\"",
            "\"raw\tcontrol\"",
        ] {
            assert!(parse(source, "test").is_err(), "{source}");
        }
        let (value, _) = parse(r#""\ud83d\ude00""#, "test").unwrap();
        let ValueKind::String(value) = value.kind else {
            panic!()
        };
        assert_eq!(value, "😀");
    }

    #[test]
    fn exact_json_byte_node_depth_container_and_string_limits() {
        assert!(parse(
            &format!("{}0", " ".repeat(MAX_DEFINITION_BYTES - 1)),
            "test"
        )
        .is_ok());
        assert!(parse(&format!("{}0", " ".repeat(MAX_DEFINITION_BYTES)), "test").is_err());
        assert!(parse(&format!("{}0{}", "[".repeat(32), "]".repeat(32)), "test").is_ok());
        assert!(parse(&format!("{}0{}", "[".repeat(33), "]".repeat(33)), "test").is_err());
        for (count, good) in [(256, true), (257, false)] {
            assert_eq!(
                parse(&serde_json::to_string(&vec![0; count]).unwrap(), "test").is_ok(),
                good
            );
            let object: BTreeMap<_, _> = (0..count).map(|i| (i.to_string(), 0)).collect();
            assert_eq!(
                parse(&serde_json::to_string(&object).unwrap(), "test").is_ok(),
                good
            );
            assert_eq!(
                parse(&serde_json::to_string(&"x".repeat(count)).unwrap(), "test").is_ok(),
                good
            );
        }
        assert!(parse(&serde_json::to_string(&"😀".repeat(64)).unwrap(), "test").is_ok());
        assert!(parse(&serde_json::to_string(&"😀".repeat(65)).unwrap(), "test").is_err());
        // 1 root +16 arrays +4079 numbers =4096 nodes. Both the individual
        // arrays and the root remain below the separate 256-entry allowance.
        let exact = format!(
            "[{}]",
            (0..16)
                .map(|i| format!("[{}]", vec!["0"; if i == 15 { 254 } else { 255 }].join(",")))
                .collect::<Vec<_>>()
                .join(",")
        );
        let (_, stats) = parse(&exact, "test").unwrap();
        assert_eq!(stats.nodes, 4096);
        let too_many = exact.replacen("[0,", "[0,0,", 1);
        assert!(matches!(
            parse(&too_many, "test").unwrap_err().kind(),
            Kind::Limit {
                limit: DensityLimit::JsonNodes,
                count: 4097,
                maximum: 4096
            }
        ));
    }
}
