//! Durable entity identity, independent of runtime and protocol handles.

use core::fmt;
use core::str::FromStr;

use uuid::Uuid;

/// The complete UUID of a persistable entity.
///
/// This immutable value preserves all 128 bits, including nil, non-v4 and
/// non-RFC-variant values. It does not generate identifiers or validate
/// uniqueness. Entity owners must enforce duplicate, lifetime and scope rules.
/// It contains no world, dimension, wire ID or internal generational handle.
/// [`crate::EntityId`] remains the signed `i32` protocol reference; it cannot
/// identify an entity durably across unloads or server restarts.
///
/// [`Display`](fmt::Display) emits lowercase hyphenated text. [`FromStr`]
/// delegates to [`Uuid::parse_str`], accepting its simple, hyphenated, braced
/// and `urn:uuid:` forms without trimming input. This is the Rust UUID parser
/// contract, not a Minecraft NBT, protocol or storage codec. Optional `serde`
/// support transparently uses the underlying UUID representation.
/// Equality, hashing and ordering delegate to [`Uuid`]; ordering compares
/// canonical bytes unsigned and does not define a Java collection-order contract.
///
/// Wire IDs cannot be passed as durable identity:
///
/// ```compile_fail,E0308
/// use ferrumc_core::{EntityId, EntityUuid};
/// fn identify_entity(_: EntityUuid) {}
/// identify_entity(EntityId::new(7));
/// ```
///
/// Player identities also require an explicit domain decision:
///
/// ```compile_fail,E0308
/// use ferrumc_core::{EntityUuid, PlayerId};
/// fn identify_entity(_: EntityUuid) {}
/// identify_entity(PlayerId::offline("Steve"));
/// ```
///
/// There are also no conversions from either ID domain:
///
/// ```compile_fail,E0277
/// use ferrumc_core::{EntityId, EntityUuid};
/// fn identify_entity(_: EntityUuid) {}
/// identify_entity(EntityId::new(7).into());
/// ```
///
/// ```compile_fail,E0277
/// use ferrumc_core::{EntityUuid, PlayerId};
/// fn identify_entity(_: EntityUuid) {}
/// identify_entity(PlayerId::offline("Steve").into());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct EntityUuid(Uuid);

impl EntityUuid {
    /// Wraps an explicitly supplied UUID without modifying any bits.
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Returns the unchanged underlying UUID.
    pub const fn as_uuid(self) -> Uuid {
        self.0
    }

    /// Preserves sixteen bytes in canonical UUID/network order.
    ///
    /// For `01234567-89ab-cdef-fedc-ba9876543210`, the first four bytes are
    /// `[0x01, 0x23, 0x45, 0x67]`. No little-endian field swapping or UUID
    /// version/variant stamping occurs. This is not an NBT int-array codec.
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(Uuid::from_bytes(bytes))
    }

    /// Returns all sixteen bytes in the order accepted by [`Self::from_bytes`].
    pub const fn to_bytes(self) -> [u8; 16] {
        self.0.into_bytes()
    }
}

impl FromStr for EntityUuid {
    type Err = uuid::Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(text).map(Self::from_uuid)
    }
}

impl fmt::Display for EntityUuid {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl From<Uuid> for EntityUuid {
    fn from(uuid: Uuid) -> Self {
        Self::from_uuid(uuid)
    }
}

impl From<EntityUuid> for Uuid {
    fn from(identity: EntityUuid) -> Self {
        identity.as_uuid()
    }
}
