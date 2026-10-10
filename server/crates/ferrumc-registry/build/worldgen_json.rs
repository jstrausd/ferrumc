//! Build-only, bounded JSON materialization with duplicate-key refusal.

use std::fmt;

use serde::de::{DeserializeSeed, Error as _, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};

#[derive(Clone, Copy)]
pub(crate) struct Limits {
    pub bytes: usize,
    pub nodes: usize,
    pub depth: usize,
    pub entries: usize,
    pub string_bytes: usize,
}

#[derive(Default, Debug, Clone, Copy)]
pub(crate) struct Stats {
    pub nodes: usize,
    pub depth: usize,
}

pub(super) fn parse(bytes: &[u8], limits: Limits) -> Result<(Value, Stats), serde_json::Error> {
    if bytes.len() > limits.bytes {
        return Err(serde_json::Error::custom("JSON byte limit exceeded"));
    }
    let mut stats = Stats::default();
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let value = Seed {
        limits,
        stats: &mut stats,
        depth: 0,
    }
    .deserialize(&mut deserializer)?;
    deserializer.end()?;
    Ok((value, stats))
}

struct Seed<'a> {
    limits: Limits,
    stats: &'a mut Stats,
    depth: usize,
}

impl Seed<'_> {
    fn charge<E: serde::de::Error>(&mut self, depth: usize) -> Result<(), E> {
        if depth > self.limits.depth || self.stats.nodes >= self.limits.nodes {
            return Err(E::custom("JSON node/depth limit exceeded"));
        }
        self.stats.nodes += 1;
        self.stats.depth = self.stats.depth.max(depth);
        Ok(())
    }
}

impl<'de> DeserializeSeed<'de> for Seed<'_> {
    type Value = Value;

    fn deserialize<D: serde::Deserializer<'de>>(
        mut self,
        deserializer: D,
    ) -> Result<Value, D::Error> {
        self.charge(self.depth)?;
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Seed<'_> {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("bounded JSON with unique keys and finite numbers")
    }

    fn visit_bool<E: serde::de::Error>(self, value: bool) -> Result<Value, E> {
        Ok(Value::Bool(value))
    }
    fn visit_unit<E: serde::de::Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }
    fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }
    fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Value, E> {
        Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }
    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Value, E> {
        if value.len() > self.limits.string_bytes {
            return Err(E::custom("JSON string limit exceeded"));
        }
        Ok(Value::String(value.to_owned()))
    }
    fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Value, E> {
        if value.len() > self.limits.string_bytes {
            return Err(E::custom("JSON string limit exceeded"));
        }
        Ok(Value::String(value))
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Value, A::Error> {
        let mut values = Vec::new();
        loop {
            // The seed fails before materializing an element beyond the cap.
            let limits = if values.len() == self.limits.entries {
                Limits {
                    nodes: self.stats.nodes,
                    ..self.limits
                }
            } else {
                self.limits
            };
            let Some(value) = sequence.next_element_seed(Seed {
                limits,
                stats: self.stats,
                depth: self.depth + 1,
            })?
            else {
                break;
            };
            values.push(value);
        }
        Ok(Value::Array(values))
    }
    fn visit_map<A: MapAccess<'de>>(mut self, mut map: A) -> Result<Value, A::Error> {
        let mut values = Map::new();
        while let Some(key) = map.next_key_seed(Text {
            limit: self.limits.string_bytes,
        })? {
            if values.len() == self.limits.entries {
                return Err(A::Error::custom("JSON object entry limit exceeded"));
            }
            self.charge(self.depth + 1)?;
            if values.contains_key(&key) {
                return Err(A::Error::custom("duplicate JSON key"));
            }
            let value = map.next_value_seed(Seed {
                limits: self.limits,
                stats: self.stats,
                depth: self.depth + 1,
            })?;
            values.insert(key, value);
        }
        Ok(Value::Object(values))
    }
}

struct Text {
    limit: usize,
}
impl<'de> DeserializeSeed<'de> for Text {
    type Value = String;
    fn deserialize<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<String, D::Error> {
        deserializer.deserialize_str(self)
    }
}
impl Visitor<'_> for Text {
    type Value = String;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("bounded JSON key")
    }
    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<String, E> {
        if value.len() > self.limit {
            return Err(E::custom("JSON key limit exceeded"));
        }
        Ok(value.to_owned())
    }
    fn visit_string<E: serde::de::Error>(self, value: String) -> Result<String, E> {
        if value.len() > self.limit {
            return Err(E::custom("JSON key limit exceeded"));
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::{parse, Limits};
    const LIMITS: Limits = Limits {
        bytes: 128 * 1024,
        nodes: 4096,
        depth: 32,
        entries: 256,
        string_bytes: 256,
    };

    #[test]
    fn array_object_string_node_depth_and_byte_bounds_are_enforced() {
        let full = serde_json::to_vec(&vec![0; 256]).unwrap();
        assert!(parse(&full, LIMITS).is_ok());
        assert!(parse(&serde_json::to_vec(&vec![0; 257]).unwrap(), LIMITS).is_err());
        let object: std::collections::BTreeMap<_, _> =
            (0..257).map(|i| (format!("key{i}"), 0)).collect();
        assert!(parse(&serde_json::to_vec(&object).unwrap(), LIMITS).is_err());
        assert!(parse(&serde_json::to_vec(&"s".repeat(256)).unwrap(), LIMITS).is_ok());
        assert!(parse(&serde_json::to_vec(&"s".repeat(257)).unwrap(), LIMITS).is_err());
        let deep = format!("{}0{}", "[".repeat(33), "]".repeat(33));
        assert!(parse(deep.as_bytes(), LIMITS)
            .unwrap_err()
            .to_string()
            .contains("node/depth"));
        let many = serde_json::to_vec(&vec![vec![0; 16]; 256]).unwrap();
        assert!(parse(&many, LIMITS)
            .unwrap_err()
            .to_string()
            .contains("node/depth"));
        assert!(parse(&vec![b' '; 128 * 1024 + 1], LIMITS)
            .unwrap_err()
            .to_string()
            .contains("byte limit"));
    }

    #[test]
    fn escaped_duplicate_keys_and_nonfinite_numbers_are_refused() {
        assert!(parse(br#"{"a":0,"\u0061":1}"#, LIMITS)
            .unwrap_err()
            .to_string()
            .contains("duplicate JSON key"));
        for input in [
            b"1e309".as_slice(),
            b"NaN",
            b"Infinity",
            b"-Infinity",
            b"{\"a\":1,}",
        ] {
            assert!(parse(input, LIMITS).is_err());
        }
        let (_, stats) = parse(br#"{"a":[0,1]}"#, LIMITS).unwrap();
        assert_eq!(stats.nodes, 5);
        assert_eq!(stats.depth, 2);
    }
}
