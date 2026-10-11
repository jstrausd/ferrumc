//! Full strict JSON syntax validation with a private climate projection.
//! Borrowed key spans preserve decoded identity without a DOM or heap strings.

use super::{
    check_limit, BiomeTemperatureError as Error, BiomeTemperatureLimit as Limit, Climate, Kind,
};

const SOURCE_MAX: usize = 128 * 1024;
const NODES_MAX: usize = 4096;
const DEPTH_MAX: usize = 32;
const ENTRIES_MAX: usize = 256;
const STRING_MAX: usize = 256;

// Includes the parser, conservative move/initialization copies, a current frame,
// both decode buffers, candidates, complete table/noise staging and constructor
// temporaries. This is native object capacity, not allocator overhead or RSS.
pub(super) const SCRATCH_BYTES: usize = 8 * std::mem::size_of::<Parser<'static>>()
    + 3 * std::mem::size_of::<Frame>()
    + 4 * std::mem::size_of::<super::BuiltinBiomeTemperatures>()
    + 5 * std::mem::size_of::<super::SimplexNoise>()
    + 4 * std::mem::size_of::<super::WorldgenRandomSource>()
    + 4096;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Stats {
    pub nodes: usize,
    pub depth: usize,
}

#[derive(Clone, Copy)]
struct Span {
    start: u32,
    end: u32,
}

#[derive(Clone, Copy)]
enum State {
    ObjectFirst,
    ObjectKey,
    ObjectAfter,
    ArrayFirst,
    ArrayValue,
    ArrayAfter,
}

#[derive(Clone, Copy)]
struct Frame {
    state: State,
    entries: usize,
    keys: [Span; ENTRIES_MAX],
}

impl Frame {
    const EMPTY: Self = Self {
        state: State::ObjectFirst,
        entries: 0,
        keys: [Span { start: 0, end: 0 }; ENTRIES_MAX],
    };
}

struct Parser<'a> {
    input: &'a str,
    position: usize,
    stats: Stats,
    frames: [Frame; DEPTH_MAX + 1],
    frames_live: usize,
    decoded: [u8; STRING_MAX],
    previous: [u8; STRING_MAX],
    temperature: Option<f32>,
    precipitation: Option<bool>,
    frozen: bool,
}

pub(super) fn check_source_bytes(bytes: usize) -> Result<(), Error> {
    check_limit(bytes, SOURCE_MAX, Limit::SourceBytes)
}

pub(super) fn parse(input: &str) -> Result<(Climate, Stats), Error> {
    let mut parser = Parser::new(input)?;
    parser.scan()?;
    let temperature = parser.temperature.ok_or_else(|| {
        parser
            .error(Kind::MissingClimateField)
            .at_source(parser.position, Some("temperature"))
    })?;
    let has_precipitation = parser.precipitation.ok_or_else(|| {
        parser
            .error(Kind::MissingClimateField)
            .at_source(parser.position, Some("has_precipitation"))
    })?;
    Ok((
        Climate {
            temperature,
            has_precipitation,
            frozen: parser.frozen,
        },
        parser.stats,
    ))
}

impl<'a> Parser<'a> {
    // The approved allocation-free contract requires this fixed frame array.
    // Its complete native capacity and Debug/Release stack envelope are tested.
    #[allow(clippy::large_stack_arrays)]
    fn new(input: &'a str) -> Result<Self, Error> {
        check_source_bytes(input.len())?;
        Ok(Self {
            input,
            position: 0,
            stats: Stats::default(),
            frames: [Frame::EMPTY; DEPTH_MAX + 1],
            frames_live: 0,
            decoded: [0; STRING_MAX],
            previous: [0; STRING_MAX],
            temperature: None,
            precipitation: None,
            frozen: false,
        })
    }

    fn error(&self, kind: Kind) -> Error {
        Error::new(kind).at_source(self.position, None)
    }
    fn check(&self, count: usize, maximum: usize, limit: Limit) -> Result<(), Error> {
        check_limit(count, maximum, limit).map_err(|e| e.at_source(self.position, None))
    }
    fn charge(&mut self, depth: usize) -> Result<(), Error> {
        self.check(depth, DEPTH_MAX, Limit::JsonDepth)?;
        self.check(self.stats.nodes + 1, NODES_MAX, Limit::JsonNodes)?;
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
    fn literal(&mut self, value: &[u8]) -> Result<(), Error> {
        if self.input.as_bytes()[self.position..].starts_with(value) {
            self.position += value.len();
            Ok(())
        } else {
            Err(self.error(Kind::InvalidJson))
        }
    }
    fn number(&mut self) -> Result<&'a str, Error> {
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
        Ok(&self.input[start..self.position])
    }

    fn value(&mut self, depth: usize, field: Option<&'static str>) -> Result<(), Error> {
        self.whitespace();
        self.charge(depth)?;
        let start = self.position;
        let byte = self.byte();
        let invalid = || Error::new(Kind::InvalidClimateField).at_source(start, field);
        match field {
            Some("temperature") => {
                if !matches!(byte, Some(b'-' | b'0'..=b'9')) {
                    return Err(invalid());
                }
                let token = self.number()?;
                let value = token.parse::<f32>().map_err(|_| invalid())?;
                if !value.is_finite() {
                    return Err(invalid());
                }
                self.temperature = Some(value);
            }
            Some("has_precipitation") => {
                let value = match byte {
                    Some(b't') => {
                        self.literal(b"true")?;
                        true
                    }
                    Some(b'f') => {
                        self.literal(b"false")?;
                        false
                    }
                    _ => return Err(invalid()),
                };
                self.precipitation = Some(value);
            }
            Some("temperature_modifier") => {
                if byte != Some(b'"') {
                    return Err(invalid());
                }
                let count = decode_string(self.input, &mut self.position, &mut self.decoded)?;
                self.frozen = match &self.decoded[..count] {
                    b"none" => false,
                    b"frozen" => true,
                    _ => return Err(invalid()),
                };
            }
            _ => match byte {
                Some(b'n') => self.literal(b"null")?,
                Some(b't') => self.literal(b"true")?,
                Some(b'f') => self.literal(b"false")?,
                Some(b'"') => {
                    decode_string(self.input, &mut self.position, &mut self.decoded)?;
                }
                Some(b'-' | b'0'..=b'9') => {
                    self.number()?;
                }
                Some(b'{' | b'[') => {
                    self.position += 1;
                    self.frames[depth].state = if byte == Some(b'{') {
                        State::ObjectFirst
                    } else {
                        State::ArrayFirst
                    };
                    self.frames[depth].entries = 0;
                    self.frames_live = depth + 1;
                }
                _ => return Err(self.error(Kind::InvalidJson)),
            },
        }
        Ok(())
    }

    fn key(&mut self, depth: usize) -> Result<Option<&'static str>, Error> {
        let count = self.frames[depth].entries;
        self.check(count + 1, ENTRIES_MAX, Limit::ContainerEntries)?;
        self.charge(depth + 1)?;
        let start = self.position;
        let length = decode_string(self.input, &mut self.position, &mut self.decoded)?;
        for prior in self.frames[depth].keys[..count].iter().copied() {
            let mut position = prior.start as usize;
            let previous = decode_string(self.input, &mut position, &mut self.previous)?;
            if previous == length && self.previous[..previous] == self.decoded[..length] {
                return Err(Error::new(Kind::DuplicateField).at_source(start, None));
            }
            // Span endpoints are retained, so decoding never silently crosses
            // into another token if this private representation changes.
            if position != prior.end as usize {
                return Err(self.error(Kind::InvalidJson));
            }
        }
        self.frames[depth].keys[count] = Span {
            start: start as u32,
            end: self.position as u32,
        };
        self.frames[depth].entries += 1;
        let field = if depth == 0 {
            match &self.decoded[..length] {
                b"temperature" => Some("temperature"),
                b"has_precipitation" => Some("has_precipitation"),
                b"temperature_modifier" => Some("temperature_modifier"),
                _ => None,
            }
        } else {
            None
        };
        Ok(field)
    }

    fn scan(&mut self) -> Result<(), Error> {
        self.whitespace();
        if self.byte() != Some(b'{') {
            return Err(self.error(Kind::InvalidClimateField));
        }
        self.value(0, None)?;
        while self.frames_live != 0 {
            let depth = self.frames_live - 1;
            self.whitespace();
            match self.frames[depth].state {
                State::ObjectFirst | State::ObjectKey => {
                    if matches!(self.frames[depth].state, State::ObjectFirst) && self.eat(b'}') {
                        self.frames_live -= 1;
                        continue;
                    }
                    let field = self.key(depth)?;
                    self.whitespace();
                    if !self.eat(b':') {
                        return Err(self.error(Kind::InvalidJson));
                    }
                    self.frames[depth].state = State::ObjectAfter;
                    self.value(depth + 1, field)?;
                }
                State::ObjectAfter => {
                    if self.eat(b'}') {
                        self.frames_live -= 1;
                    } else if self.eat(b',') {
                        self.frames[depth].state = State::ObjectKey;
                    } else {
                        return Err(self.error(Kind::InvalidJson));
                    }
                }
                State::ArrayFirst | State::ArrayValue => {
                    if matches!(self.frames[depth].state, State::ArrayFirst) && self.eat(b']') {
                        self.frames_live -= 1;
                        continue;
                    }
                    self.check(
                        self.frames[depth].entries + 1,
                        ENTRIES_MAX,
                        Limit::ContainerEntries,
                    )?;
                    self.frames[depth].entries += 1;
                    self.frames[depth].state = State::ArrayAfter;
                    self.value(depth + 1, None)?;
                }
                State::ArrayAfter => {
                    if self.eat(b']') {
                        self.frames_live -= 1;
                    } else if self.eat(b',') {
                        self.frames[depth].state = State::ArrayValue;
                    } else {
                        return Err(self.error(Kind::InvalidJson));
                    }
                }
            }
        }
        self.whitespace();
        if self.position != self.input.len() {
            return Err(self.error(Kind::InvalidJson));
        }
        Ok(())
    }
}

fn decode_string(
    input: &str,
    position: &mut usize,
    decoded: &mut [u8; STRING_MAX],
) -> Result<usize, Error> {
    let bytes = input.as_bytes();
    let mut length = 0;
    if bytes.get(*position) != Some(&b'"') {
        return Err(Error::new(Kind::InvalidJson).at_source(*position, None));
    }
    *position += 1;
    loop {
        let byte = take(bytes, position)?;
        match byte {
            b'"' => return Ok(length),
            0..=31 => return Err(Error::new(Kind::InvalidJson).at_source(*position - 1, None)),
            b'\\' => {
                let escaped = take(bytes, position)?;
                let value = match escaped {
                    b'"' | b'\\' | b'/' => escaped,
                    b'b' => 8,
                    b'f' => 12,
                    b'n' => b'\n',
                    b'r' => b'\r',
                    b't' => b'\t',
                    b'u' => {
                        let first = hex4(bytes, position)?;
                        let scalar = if (0xd800..=0xdbff).contains(&first) {
                            if take(bytes, position)? != b'\\' || take(bytes, position)? != b'u' {
                                return Err(
                                    Error::new(Kind::InvalidJson).at_source(*position, None)
                                );
                            }
                            let second = hex4(bytes, position)?;
                            if !(0xdc00..=0xdfff).contains(&second) {
                                return Err(
                                    Error::new(Kind::InvalidJson).at_source(*position, None)
                                );
                            }
                            0x10000 + ((first - 0xd800) << 10) + second - 0xdc00
                        } else {
                            first
                        };
                        let character = char::from_u32(scalar).ok_or_else(|| {
                            Error::new(Kind::InvalidJson).at_source(*position, None)
                        })?;
                        let mut buffer = [0; 4];
                        for &part in character.encode_utf8(&mut buffer).as_bytes() {
                            push(decoded, &mut length, part, *position)?;
                        }
                        continue;
                    }
                    _ => return Err(Error::new(Kind::InvalidJson).at_source(*position - 1, None)),
                };
                push(decoded, &mut length, value, *position)?;
            }
            value => push(decoded, &mut length, value, *position)?,
        }
    }
}

fn take(bytes: &[u8], position: &mut usize) -> Result<u8, Error> {
    let byte = bytes
        .get(*position)
        .copied()
        .ok_or_else(|| Error::new(Kind::InvalidJson).at_source(*position, None))?;
    *position += 1;
    Ok(byte)
}

fn hex4(bytes: &[u8], position: &mut usize) -> Result<u32, Error> {
    let mut value = 0;
    for _ in 0..4 {
        let byte = take(bytes, position)?;
        let digit = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            b'A'..=b'F' => byte - b'A' + 10,
            _ => return Err(Error::new(Kind::InvalidJson).at_source(*position - 1, None)),
        };
        value = value * 16 + u32::from(digit);
    }
    Ok(value)
}

fn push(
    decoded: &mut [u8; STRING_MAX],
    length: &mut usize,
    byte: u8,
    position: usize,
) -> Result<(), Error> {
    check_limit(*length + 1, STRING_MAX, Limit::StringBytes)
        .map_err(|e| e.at_source(position, None))?;
    decoded[*length] = byte;
    *length += 1;
    Ok(())
}

#[cfg(test)]
mod tests;
