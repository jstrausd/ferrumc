//! The declarative packet spec: the parsed, validated form of
//! `docs/protocol/1_21_8/packets.toml`.
//!
//! This module owns only the *model* and its loader/validator. Turning the model
//! into Rust source lives in [`crate::emit`]. Keeping the two apart means the
//! parser can be unit-tested without going through `rustfmt`, and the emitter can
//! assume it is handed an already-valid spec.

use std::collections::BTreeSet;
use std::path::Path;

use serde::Deserialize;

use crate::error::GenError;

/// Resource-location strings (`identifier`) are capped at this many UTF-16 code
/// units — the Minecraft string maximum. Modeled as a `BoundedString` of this
/// size so identifiers ride the same hostile-input guard as ordinary strings.
pub(crate) const IDENTIFIER_MAX: usize = 32767;

/// Largest admitted opaque unprefixed tail; mirrors the existing 2-MiB blob cap.
pub(crate) const MAX_BOUNDED_REMAINING_BYTES: usize = 2_097_152;

/// A connection state. Ordering follows the protocol's own progression so the
/// emitted modules sort deterministically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum State {
    /// The initial handshaking state.
    Handshake,
    /// Server list ping.
    Status,
    /// Offline/online login negotiation.
    Login,
    /// The 1.20.2+ configuration state between login and play.
    Configuration,
    /// The play state: the in-game world session.
    Play,
}

impl State {
    /// Parses the spec's lowercase state name.
    fn parse(raw: &str) -> Result<Self, GenError> {
        match raw {
            "handshake" => Ok(Self::Handshake),
            "status" => Ok(Self::Status),
            "login" => Ok(Self::Login),
            "configuration" => Ok(Self::Configuration),
            "play" => Ok(Self::Play),
            other => Err(GenError::PacketsInvalid(format!("unknown state `{other}`"))),
        }
    }

    /// The generated module name for this state (e.g. `configuration`).
    pub(crate) fn module(self) -> &'static str {
        match self {
            Self::Handshake => "handshake",
            Self::Status => "status",
            Self::Login => "login",
            Self::Configuration => "configuration",
            Self::Play => "play",
        }
    }

    /// The `crate::State` variant name used in generated code.
    pub(crate) fn variant(self) -> &'static str {
        match self {
            Self::Handshake => "Handshake",
            Self::Status => "Status",
            Self::Login => "Login",
            Self::Configuration => "Configuration",
            Self::Play => "Play",
        }
    }
}

/// The direction a packet travels. `Serverbound` sorts before `Clientbound` so
/// the emitted dispatch enums have a stable order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Direction {
    /// Client to server.
    Serverbound,
    /// Server to client.
    Clientbound,
}

impl Direction {
    /// Parses the spec's lowercase direction name.
    fn parse(raw: &str) -> Result<Self, GenError> {
        match raw {
            "serverbound" => Ok(Self::Serverbound),
            "clientbound" => Ok(Self::Clientbound),
            other => Err(GenError::PacketsInvalid(format!(
                "unknown direction `{other}`"
            ))),
        }
    }

    /// The `crate::Direction` variant name (also the dispatch-enum name prefix).
    pub(crate) fn variant(self) -> &'static str {
        match self {
            Self::Serverbound => "Serverbound",
            Self::Clientbound => "Clientbound",
        }
    }
}

/// A single wire type from the spec's closed type grammar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WireType {
    /// `varint`: a signed 32-bit `VarInt` (`LEB128`-style).
    VarInt,
    /// `varlong`: a signed 64-bit `VarLong` (`LEB128`-style).
    VarLong,
    /// `u16`: a fixed-width big-endian unsigned 16-bit integer.
    U16,
    /// `i16`: a fixed-width big-endian signed 16-bit integer.
    I16,
    /// `i32`: a fixed-width big-endian signed 32-bit integer.
    I32,
    /// `u32`: a fixed-width big-endian unsigned 32-bit integer (used for the
    /// play-state teleport-relative bit mask).
    U32,
    /// `i64`: a fixed-width big-endian signed 64-bit integer.
    I64,
    /// `i8`: a signed byte.
    I8,
    /// `u8`: an unsigned byte.
    U8,
    /// `f32`: a big-endian IEEE-754 32-bit float.
    F32,
    /// `f64`: a big-endian IEEE-754 64-bit float.
    F64,
    /// `bool`: a single 0/1 byte.
    Bool,
    /// `uuid`: 16 big-endian bytes.
    Uuid,
    /// `position`: a block position packed into a big-endian `i64`
    /// (26-bit x, 26-bit z, 12-bit y), decoded to a typed [`BlockPosition`].
    ///
    /// [`BlockPosition`]: ../../ferrumc_proto/types/struct.BlockPosition.html
    Position,
    /// `string(N)`: a length-prefixed UTF-8 string capped at `N` code units.
    Str(usize),
    /// `identifier`: a resource location (a string capped at [`IDENTIFIER_MAX`]).
    Identifier,
    /// `nbt`: a network-form (unnamed-root) NBT compound.
    Nbt,
    /// `prefixed_bytes(N)`: a `VarInt` byte-length prefix then that many raw
    /// bytes, capped at `N` (a `BoundedBytes<N>`). Used to carry the heavy chunk
    /// payload and per-section light arrays as opaque, length-delimited blobs.
    PrefixedBytes(usize),
    /// `remaining_bytes`: every byte left in the packet body, verbatim. Must be
    /// the final field of a packet; used for variable trailing payloads (the
    /// Player Info Update entry list) the typed grammar cannot yet express.
    RemainingBytes,
    /// `bounded_remaining_bytes(N)`: at most N raw body bytes, without a prefix.
    /// Only a packet's direct final field may own the remaining body.
    BoundedRemainingBytes(usize),
    /// `optional<T>`: a bool flag followed by `T` only when the flag is set.
    Optional(Box<WireType>),
    /// `prefixed_array<T>`: a `VarInt` count followed by that many `T`.
    PrefixedArray(Box<WireType>),
    /// A reference to a composite payload declared in a `[[struct]]` table.
    Struct(String),
}

impl WireType {
    /// Parses one type expression, validating struct references against the set
    /// of declared struct names.
    fn parse(raw: &str, structs: &BTreeSet<String>) -> Result<Self, GenError> {
        let raw = raw.trim();

        if let Some(inner) = wrapped(raw, "optional<", ">") {
            return Ok(Self::Optional(Box::new(Self::parse(inner, structs)?)));
        }
        if let Some(inner) = wrapped(raw, "prefixed_array<", ">") {
            return Ok(Self::PrefixedArray(Box::new(Self::parse(inner, structs)?)));
        }
        if let Some(arg) = wrapped(raw, "string(", ")") {
            let max = arg
                .trim()
                .parse::<usize>()
                .map_err(|_| GenError::PacketsInvalid(format!("invalid string size in `{raw}`")))?;
            return Ok(Self::Str(max));
        }
        if let Some(arg) = wrapped(raw, "prefixed_bytes(", ")") {
            let max = arg.trim().parse::<usize>().map_err(|_| {
                GenError::PacketsInvalid(format!("invalid prefixed_bytes size in `{raw}`"))
            })?;
            return Ok(Self::PrefixedBytes(max));
        }
        if let Some(arg) = wrapped(raw, "bounded_remaining_bytes(", ")") {
            let arg = arg.trim();
            if arg.is_empty() || !arg.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(GenError::PacketsInvalid(format!(
                    "bounded_remaining_bytes requires an ASCII decimal cap in `{raw}`"
                )));
            }
            let max = arg.parse::<usize>().map_err(|_| {
                GenError::PacketsInvalid(format!(
                    "overflowing bounded_remaining_bytes cap in `{raw}`"
                ))
            })?;
            if max > MAX_BOUNDED_REMAINING_BYTES {
                return Err(GenError::PacketsInvalid(format!(
                    "bounded_remaining_bytes cap {max} exceeds {MAX_BOUNDED_REMAINING_BYTES}"
                )));
            }
            return Ok(Self::BoundedRemainingBytes(max));
        }

        Ok(match raw {
            "varint" => Self::VarInt,
            "varlong" => Self::VarLong,
            "u16" => Self::U16,
            "i16" => Self::I16,
            "i32" => Self::I32,
            "u32" => Self::U32,
            "i64" => Self::I64,
            "i8" => Self::I8,
            "u8" => Self::U8,
            "f32" => Self::F32,
            "f64" => Self::F64,
            "bool" => Self::Bool,
            "uuid" => Self::Uuid,
            "position" => Self::Position,
            "identifier" => Self::Identifier,
            "nbt" => Self::Nbt,
            "remaining_bytes" => Self::RemainingBytes,
            other if structs.contains(other) => Self::Struct(other.to_owned()),
            other => {
                return Err(GenError::PacketsInvalid(format!(
                    "unknown wire type `{other}`"
                )))
            }
        })
    }

    /// The Rust type this wire type decodes to.
    pub(crate) fn rust_type(&self) -> String {
        match self {
            Self::VarInt | Self::I32 => "i32".to_owned(),
            Self::VarLong | Self::I64 => "i64".to_owned(),
            Self::U16 => "u16".to_owned(),
            Self::I16 => "i16".to_owned(),
            Self::U32 => "u32".to_owned(),
            Self::I8 => "i8".to_owned(),
            Self::U8 => "u8".to_owned(),
            Self::F32 => "f32".to_owned(),
            Self::F64 => "f64".to_owned(),
            Self::Bool => "bool".to_owned(),
            Self::Uuid => "uuid::Uuid".to_owned(),
            Self::Position => "crate::types::BlockPosition".to_owned(),
            Self::Str(max) => format!("BoundedString<{}>", group_digits(*max)),
            Self::Identifier => format!("BoundedString<{}>", group_digits(IDENTIFIER_MAX)),
            Self::Nbt => "ferrumc_nbt::NbtTag".to_owned(),
            Self::PrefixedBytes(max) | Self::BoundedRemainingBytes(max) => {
                format!("ferrumc_codec::BoundedBytes<{}>", group_digits(*max))
            }
            Self::RemainingBytes => "Vec<u8>".to_owned(),
            Self::Optional(inner) => format!("Option<{}>", inner.rust_type()),
            Self::PrefixedArray(inner) => format!("Vec<{}>", inner.rust_type()),
            Self::Struct(name) => name.clone(),
        }
    }

    /// Whether the decoded Rust type is `Copy` (decides getter style: by value
    /// for `Copy`, by reference otherwise).
    pub(crate) fn is_copy(&self) -> bool {
        matches!(
            self,
            Self::VarInt
                | Self::VarLong
                | Self::U16
                | Self::I16
                | Self::I32
                | Self::U32
                | Self::I64
                | Self::I8
                | Self::U8
                | Self::F32
                | Self::F64
                | Self::Bool
                | Self::Uuid
                | Self::Position
        )
    }

    /// Whether this expression hides or directly owns an unprefixed bounded tail.
    fn contains_bounded_remaining(&self) -> bool {
        match self {
            Self::BoundedRemainingBytes(_) => true,
            Self::Optional(inner) | Self::PrefixedArray(inner) => {
                inner.contains_bounded_remaining()
            }
            _ => false,
        }
    }

    /// Finds legacy raw tails spelled within this field expression. Named
    /// composite contents keep their pre-existing admission in this ticket.
    fn contains_visible_legacy_tail(&self) -> bool {
        match self {
            Self::RemainingBytes => true,
            Self::Optional(inner) | Self::PrefixedArray(inner) => {
                inner.contains_visible_legacy_tail()
            }
            _ => false,
        }
    }
}

/// Formats an integer literal with `_` thousands separators (for any value of 5
/// digits or more), so generated const-generic sizes read clearly and satisfy
/// `clippy::unreadable_literal`.
pub(crate) fn group_digits(n: usize) -> String {
    let s = n.to_string();
    if s.len() <= 4 {
        return s;
    }
    // Chunk three digits at a time from the right (reverse, chunk, reverse back)
    // to avoid any modular arithmetic in the index math.
    let reversed: Vec<char> = s.chars().rev().collect();
    let mut groups: Vec<String> = reversed
        .chunks(3)
        .map(|chunk| chunk.iter().rev().collect())
        .collect();
    groups.reverse();
    groups.join("_")
}

/// Strips a `prefix`/`suffix` pair, returning the inner text if both match.
fn wrapped<'a>(raw: &'a str, prefix: &str, suffix: &str) -> Option<&'a str> {
    raw.strip_prefix(prefix)
        .and_then(|r| r.strip_suffix(suffix))
}

/// A named field inside a packet or struct.
#[derive(Debug, Clone)]
pub(crate) struct Field {
    /// Field name (`snake_case`, as written in the spec).
    pub(crate) name: String,
    /// The field's wire type.
    pub(crate) ty: WireType,
}

/// A composite payload shared by packets (and possibly other structs).
#[derive(Debug, Clone)]
pub(crate) struct StructDef {
    /// `PascalCase` type name.
    pub(crate) name: String,
    /// Fields in wire order.
    pub(crate) fields: Vec<Field>,
}

/// A single packet definition.
#[derive(Debug, Clone)]
pub(crate) struct PacketDef {
    /// The state this packet belongs to.
    pub(crate) state: State,
    /// The direction this packet travels.
    pub(crate) direction: Direction,
    /// `PascalCase` packet type name.
    pub(crate) name: String,
    /// Wire packet id for protocol 772.
    pub(crate) id: i32,
    /// Fields in wire order.
    pub(crate) fields: Vec<Field>,
}

/// The whole validated packet spec: shared structs plus every packet, both
/// sorted into a deterministic order.
#[derive(Debug, Clone)]
pub(crate) struct PacketSpec {
    /// Shared composite payloads, sorted by name.
    pub(crate) structs: Vec<StructDef>,
    /// Packets, sorted by `(state, direction, id)`.
    pub(crate) packets: Vec<PacketDef>,
}

impl PacketSpec {
    /// Loads and validates the declarative packet spec from `path`.
    ///
    /// Fails with [`GenError::PacketsRead`]/[`GenError::PacketsParse`] for a
    /// missing or malformed file, and [`GenError::PacketsInvalid`] for a
    /// semantic problem (unknown type/state/direction, duplicate id, dangling
    /// struct reference).
    pub(crate) fn load(path: &Path) -> Result<Self, GenError> {
        let raw = std::fs::read_to_string(path).map_err(|source| GenError::PacketsRead {
            path: path.to_path_buf(),
            source,
        })?;
        let doc: RawDoc = toml::from_str(&raw).map_err(|source| GenError::PacketsParse {
            path: path.to_path_buf(),
            source,
        })?;
        Self::from_raw(doc)
    }

    /// Validates a parsed document into the sorted, checked model.
    fn from_raw(doc: RawDoc) -> Result<Self, GenError> {
        // Collect struct names first so field types can reference them.
        let mut names = BTreeSet::new();
        for s in &doc.structs {
            if !names.insert(s.name.clone()) {
                return Err(GenError::PacketsInvalid(format!(
                    "duplicate struct `{}`",
                    s.name
                )));
            }
        }

        let mut structs = Vec::with_capacity(doc.structs.len());
        for s in doc.structs {
            structs.push(StructDef {
                fields: parse_fields(&s.name, s.fields, &names, false)?,
                name: s.name,
            });
        }
        structs.sort_by(|a, b| a.name.cmp(&b.name));

        let mut packets = Vec::with_capacity(doc.packet.len());
        let mut seen = BTreeSet::new();
        for p in doc.packet {
            let state = State::parse(&p.state)?;
            let direction = Direction::parse(&p.direction)?;
            if !seen.insert((state, direction, p.id)) {
                return Err(GenError::PacketsInvalid(format!(
                    "duplicate packet id {:#04x} for {}/{}",
                    p.id, p.state, p.direction
                )));
            }
            packets.push(PacketDef {
                state,
                direction,
                fields: parse_fields(&p.name, p.fields, &names, true)?,
                name: p.name,
                id: p.id,
            });
        }
        packets.sort_by_key(|packet| (packet.state, packet.direction, packet.id));

        Ok(Self { structs, packets })
    }
}

/// Parses a list of raw fields, attaching the owning item's name to any error.
fn parse_fields(
    owner: &str,
    raw: Vec<RawField>,
    structs: &BTreeSet<String>,
    packet: bool,
) -> Result<Vec<Field>, GenError> {
    let fields: Vec<Field> = raw
        .into_iter()
        .map(|f| {
            let ty = WireType::parse(&f.ty, structs)
                .map_err(|e| GenError::PacketsInvalid(format!("{owner}.{}: {e}", f.name)))?;
            Ok(Field { name: f.name, ty })
        })
        .collect::<Result<_, _>>()?;
    for (index, field) in fields.iter().enumerate() {
        if field.ty.contains_bounded_remaining()
            && (!packet
                || index + 1 != fields.len()
                || !matches!(field.ty, WireType::BoundedRemainingBytes(_)))
        {
            return Err(GenError::PacketsInvalid(format!(
                "{owner}.{}: bounded_remaining_bytes must be a direct final packet field; structs and nested types are forbidden",
                field.name
            )));
        }
    }
    if matches!(
        fields.last().map(|field| &field.ty),
        Some(WireType::BoundedRemainingBytes(_))
    ) {
        for field in &fields {
            if field.ty.contains_visible_legacy_tail() {
                return Err(GenError::PacketsInvalid(format!(
                    "{owner}.{}: remaining_bytes cannot precede bounded_remaining_bytes in the same packet; nested field expressions are included",
                    field.name
                )));
            }
        }
    }
    Ok(fields)
}

/// The raw `packets.toml` document, before validation.
#[derive(Debug, Deserialize)]
struct RawDoc {
    #[serde(default, rename = "struct")]
    structs: Vec<RawStruct>,
    #[serde(default)]
    packet: Vec<RawPacket>,
}

/// A raw `[[struct]]` table.
#[derive(Debug, Deserialize)]
struct RawStruct {
    name: String,
    #[serde(default)]
    fields: Vec<RawField>,
}

/// A raw `[[packet]]` table.
#[derive(Debug, Deserialize)]
struct RawPacket {
    state: String,
    direction: String,
    name: String,
    id: i32,
    #[serde(default)]
    fields: Vec<RawField>,
}

/// A raw inline field table (`{ name = "...", type = "..." }`).
#[derive(Debug, Deserialize)]
struct RawField {
    name: String,
    #[serde(rename = "type")]
    ty: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names() -> BTreeSet<String> {
        ["Property", "KnownPack"]
            .into_iter()
            .map(String::from)
            .collect()
    }

    #[test]
    fn parses_scalar_and_composite_types() {
        let s = names();
        assert_eq!(WireType::parse("varint", &s).unwrap(), WireType::VarInt);
        assert_eq!(WireType::parse("uuid", &s).unwrap(), WireType::Uuid);
        assert_eq!(
            WireType::parse("string(16)", &s).unwrap(),
            WireType::Str(16)
        );
        assert_eq!(
            WireType::parse("optional<string(1024)>", &s).unwrap(),
            WireType::Optional(Box::new(WireType::Str(1024)))
        );
        assert_eq!(
            WireType::parse("prefixed_array<Property>", &s).unwrap(),
            WireType::PrefixedArray(Box::new(WireType::Struct("Property".to_owned())))
        );
        assert_eq!(
            WireType::parse("optional<nbt>", &s).unwrap(),
            WireType::Optional(Box::new(WireType::Nbt))
        );
    }

    #[test]
    fn parses_play_state_wire_types() {
        let s = names();
        assert_eq!(WireType::parse("i32", &s).unwrap(), WireType::I32);
        assert_eq!(WireType::parse("u32", &s).unwrap(), WireType::U32);
        assert_eq!(WireType::parse("i16", &s).unwrap(), WireType::I16);
        assert_eq!(WireType::parse("f32", &s).unwrap(), WireType::F32);
        assert_eq!(WireType::parse("f64", &s).unwrap(), WireType::F64);
        assert_eq!(WireType::parse("position", &s).unwrap(), WireType::Position);
        assert_eq!(
            WireType::parse("remaining_bytes", &s).unwrap(),
            WireType::RemainingBytes
        );
        assert_eq!(
            WireType::parse("prefixed_bytes(2048)", &s).unwrap(),
            WireType::PrefixedBytes(2048)
        );
        assert_eq!(
            WireType::parse("prefixed_array<prefixed_bytes(2048)>", &s).unwrap(),
            WireType::PrefixedArray(Box::new(WireType::PrefixedBytes(2048)))
        );
        // f32/f64 map to non-Eq Rust types; position to the hand-written struct.
        assert_eq!(WireType::F64.rust_type(), "f64");
        assert_eq!(
            WireType::Position.rust_type(),
            "crate::types::BlockPosition"
        );
        assert_eq!(
            WireType::PrefixedBytes(2048).rust_type(),
            "ferrumc_codec::BoundedBytes<2048>"
        );
        assert_eq!(WireType::RemainingBytes.rust_type(), "Vec<u8>");
    }

    #[test]
    fn bounded_remaining_admits_ascii_decimal_caps_and_whitespace() {
        let s = names();
        for (raw, cap) in [
            ("bounded_remaining_bytes(0)", 0),
            ("bounded_remaining_bytes(1)", 1),
            ("bounded_remaining_bytes(0004)", 4),
            (" \tbounded_remaining_bytes( 12 \t) \n", 12),
            (
                "bounded_remaining_bytes(2097152)",
                MAX_BOUNDED_REMAINING_BYTES,
            ),
        ] {
            let ty = WireType::parse(raw, &s).expect("decimal cap is admitted");
            assert_eq!(ty, WireType::BoundedRemainingBytes(cap), "{raw:?}");
            assert_eq!(
                ty.rust_type(),
                format!("ferrumc_codec::BoundedBytes<{}>", group_digits(cap))
            );
            assert!(!ty.is_copy());
        }
    }

    #[test]
    fn bounded_remaining_rejects_malformed_and_excessive_caps() {
        let s = names();
        for raw in [
            "bounded_remaining_bytes",
            "bounded_remaining_bytes()",
            "bounded_remaining_bytes( )",
            "bounded_remaining_bytes(1",
            "bounded_remaining_bytes(1))",
            "bounded_remaining_bytes(1)garbage",
            "bounded_remaining_bytes (1)",
            "bounded_remaining_bytes(-1)",
            "bounded_remaining_bytes(+1)",
            "bounded_remaining_bytes(0x10)",
            "bounded_remaining_bytes(1.0)",
            "bounded_remaining_bytes(1e3)",
            "bounded_remaining_bytes(1+1)",
            "bounded_remaining_bytes(1_000)",
            "bounded_remaining_bytes(1 0)",
            "bounded_remaining_bytes(１)",
            "bounded_remaining_bytes(١)",
            "bounded_remaining_bytes(2097153)",
            "bounded_remaining_bytes(9999999999999999999999999999999999999999)",
        ] {
            assert!(
                matches!(WireType::parse(raw, &s), Err(GenError::PacketsInvalid(_))),
                "must refuse {raw:?}"
            );
        }
        // An exact usize overflow must be classified on both 32- and 64-bit hosts.
        let overflow = format!("bounded_remaining_bytes({}0)", usize::MAX);
        assert!(matches!(
            WireType::parse(&overflow, &s),
            Err(GenError::PacketsInvalid(_))
        ));
    }

    #[test]
    fn bounded_remaining_requires_direct_final_packet_field() {
        for cap in [0, 1, MAX_BOUNDED_REMAINING_BYTES] {
            let raw = format!(
                r#"
                [[packet]]
                state = "play"
                direction = "clientbound"
                name = "TailPacket"
                id = 1
                fields = [
                    {{ name = "id", type = "varint" }},
                    {{ name = "tail", type = "bounded_remaining_bytes({cap})" }},
                ]
                "#
            );
            let doc = toml::from_str(&raw).expect("valid TOML");
            let spec = PacketSpec::from_raw(doc).expect("final direct tail is admitted");
            assert_eq!(
                spec.packets[0].fields[1].ty,
                WireType::BoundedRemainingBytes(cap)
            );
        }

        let invalid = [
            (
                "packet",
                "bounded_remaining_bytes(4)",
                ", { name = 'later', type = 'u8' }",
            ),
            (
                "packet",
                "bounded_remaining_bytes(4)",
                ", { name = 'other', type = 'bounded_remaining_bytes(4)' }",
            ),
            ("struct", "bounded_remaining_bytes(4)", ""),
            ("packet", "optional<bounded_remaining_bytes(4)>", ""),
            ("packet", "prefixed_array<bounded_remaining_bytes(4)>", ""),
            (
                "packet",
                "optional<prefixed_array<bounded_remaining_bytes(4)>>",
                "",
            ),
            (
                "packet",
                "prefixed_array<optional<bounded_remaining_bytes(4)>>",
                "",
            ),
            (
                "packet",
                "optional<optional<bounded_remaining_bytes(4)>>",
                "",
            ),
            (
                "packet",
                "prefixed_array<prefixed_array<bounded_remaining_bytes(4)>>",
                "",
            ),
            ("struct", "optional<bounded_remaining_bytes(4)>", ""),
            (
                "struct",
                "prefixed_array<optional<bounded_remaining_bytes(4)>>",
                "",
            ),
        ];
        for (owner, ty, after) in invalid {
            let header = if owner == "packet" {
                "state = 'play'\ndirection = 'clientbound'\nid = 1\n"
            } else {
                ""
            };
            let raw = format!(
                "[[{owner}]]\nname = 'TailOwner'\n{header}fields = [{{ name = 'tail', type = '{ty}' }}{after}]\n"
            );
            let doc = toml::from_str(&raw).expect("valid TOML");
            match PacketSpec::from_raw(doc).expect_err("placement must be refused") {
                GenError::PacketsInvalid(message) => {
                    assert!(message.contains("TailOwner.tail"), "{message}");
                    assert!(message.contains("direct final packet field"), "{message}");
                }
                other => panic!("unexpected error {other:?}"),
            }
        }
    }

    #[test]
    fn bounded_remaining_placement_does_not_change_existing_bytes_types() {
        let raw = r#"
            [[struct]]
            name = "Existing"
            fields = [
                { name = "raw", type = "remaining_bytes" },
                { name = "blobs", type = "prefixed_array<prefixed_bytes(4)>" },
            ]
            [[packet]]
            state = "play"
            direction = "clientbound"
            name = "ExistingPacket"
            id = 1
            fields = [
                { name = "composite", type = "Existing" },
                { name = "blob", type = "prefixed_bytes(0)" },
            ]
        "#;
        let doc = toml::from_str(raw).expect("valid TOML");
        PacketSpec::from_raw(doc).expect("existing admission remains unchanged");
    }

    #[test]
    fn bounded_remaining_rejects_other_visible_raw_tails_only_in_new_packets() {
        for ty in [
            "remaining_bytes",
            "optional<remaining_bytes>",
            "prefixed_array<remaining_bytes>",
            "optional<prefixed_array<remaining_bytes>>",
            "prefixed_array<optional<remaining_bytes>>",
        ] {
            let first = format!("{{ name = 'legacy', type = '{ty}' }}");
            for (last, admitted) in [
                (
                    "{ name = 'tail', type = 'bounded_remaining_bytes(4)' }",
                    false,
                ),
                ("{ name = 'scalar', type = 'u8' }", true),
            ] {
                let raw = format!(
                    "[[packet]]\nstate = 'play'\ndirection = 'clientbound'\nname = 'MixedTail'\nid = 1\nfields = [{first}, {last}]\n"
                );
                let result = PacketSpec::from_raw(toml::from_str(&raw).expect("valid TOML"));
                if admitted {
                    result.expect("legacy-only admission remains unchanged");
                } else {
                    match result.expect_err("visible second raw tail must be refused") {
                        GenError::PacketsInvalid(message) => {
                            assert!(message.contains("MixedTail.legacy"), "{message}");
                            assert!(
                                message.contains("remaining_bytes cannot precede"),
                                "{message}"
                            );
                        }
                        other => panic!("unexpected error {other:?}"),
                    }
                }
            }
        }
    }

    #[test]
    fn bounded_remaining_does_not_audit_legacy_named_composite_contents() {
        // The new type is forbidden in every struct. Existing legacy struct
        // fields are not recursively audited by this narrow grammar addition.
        let raw = r#"
            [[struct]]
            name = "LegacyRaw"
            fields = [{ name = "raw", type = "remaining_bytes" }]
            [[packet]]
            state = "play"
            direction = "clientbound"
            name = "LegacyCompositeTail"
            id = 1
            fields = [
                { name = "legacy", type = "LegacyRaw" },
                { name = "tail", type = "bounded_remaining_bytes(4)" },
            ]
        "#;
        PacketSpec::from_raw(toml::from_str(raw).expect("valid TOML"))
            .expect("legacy named composite admission is outside this ticket's audit");
    }

    #[test]
    fn rejects_unknown_type_and_dangling_struct() {
        let s = names();
        assert!(matches!(
            WireType::parse("float", &s),
            Err(GenError::PacketsInvalid(_))
        ));
        assert!(matches!(
            WireType::parse("Nope", &s),
            Err(GenError::PacketsInvalid(_))
        ));
        assert!(matches!(
            WireType::parse("string(abc)", &s),
            Err(GenError::PacketsInvalid(_))
        ));
    }

    #[test]
    fn group_digits_inserts_separators() {
        assert_eq!(group_digits(0), "0");
        assert_eq!(group_digits(64), "64");
        assert_eq!(group_digits(1024), "1024");
        assert_eq!(group_digits(32767), "32_767");
        assert_eq!(group_digits(262_144), "262_144");
        assert_eq!(group_digits(1_000_000), "1_000_000");
    }

    #[test]
    fn rust_type_mapping_is_stable() {
        assert_eq!(WireType::VarInt.rust_type(), "i32");
        assert_eq!(WireType::Str(64).rust_type(), "BoundedString<64>");
        // Large sizes get thousands separators for readability.
        assert_eq!(WireType::Str(32767).rust_type(), "BoundedString<32_767>");
        assert_eq!(
            WireType::Identifier.rust_type(),
            "BoundedString<32_767>".to_owned()
        );
        assert_eq!(
            WireType::PrefixedArray(Box::new(WireType::Uuid)).rust_type(),
            "Vec<uuid::Uuid>"
        );
    }

    #[test]
    fn from_raw_sorts_and_rejects_duplicate_ids() {
        let doc: RawDoc = toml::from_str(
            r#"
            [[packet]]
            state = "status"
            direction = "clientbound"
            name = "B"
            id = 0x01
            fields = []

            [[packet]]
            state = "status"
            direction = "clientbound"
            name = "A"
            id = 0x00
            fields = []
            "#,
        )
        .unwrap();
        let spec = PacketSpec::from_raw(doc).unwrap();
        // Sorted by id within (state, direction).
        assert_eq!(spec.packets[0].name, "A");
        assert_eq!(spec.packets[1].name, "B");

        let dup: RawDoc = toml::from_str(
            r#"
            [[packet]]
            state = "status"
            direction = "clientbound"
            name = "A"
            id = 0x00
            fields = []

            [[packet]]
            state = "status"
            direction = "clientbound"
            name = "B"
            id = 0x00
            fields = []
            "#,
        )
        .unwrap();
        assert!(matches!(
            PacketSpec::from_raw(dup),
            Err(GenError::PacketsInvalid(_))
        ));
    }

    #[test]
    fn rejects_unknown_state() {
        let doc: RawDoc = toml::from_str(
            r#"
            [[packet]]
            state = "transfer"
            direction = "clientbound"
            name = "A"
            id = 0x00
            fields = []
            "#,
        )
        .unwrap();
        assert!(matches!(
            PacketSpec::from_raw(doc),
            Err(GenError::PacketsInvalid(_))
        ));
    }
}
