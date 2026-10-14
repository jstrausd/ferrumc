//! Versioned identities for the pure, deterministic chunk-generation contract.

use ferrumc_math::ChunkPos;

use crate::{Chunk, WorldError};

/// Exact encoded length of a schema-1 [`GenerationIdentity`], in bytes.
///
/// The decoder accepts precisely this length before inspecting any fields and
/// never allocates from input. The envelope schema and generator revision are
/// independent version numbers.
pub const GENERATION_IDENTITY_LEN: usize = 27;

const IDENTITY_MAGIC: [u8; 4] = *b"FCGI";
const IDENTITY_SCHEMA: u16 = 1;
const FLAT_REVISION: u32 = 1;
const NATURAL_PREVIEW_REVISION: u32 = 1;

/// A numeric world seed with the signed 64-bit range used by Minecraft.
///
/// This value does not choose a random seed or parse a textual seed. Those are
/// configuration decisions; generation receives the already selected number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorldSeed(i64);

impl WorldSeed {
    /// Wraps an explicitly chosen numeric seed without changing its bits.
    #[must_use]
    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    /// Returns the selected signed 64-bit numeric seed.
    #[must_use]
    pub const fn value(self) -> i64 {
        self.0
    }
}

/// Minecraft release whose registry and generation rules an identity uses.
///
/// This is a closed supported set, not a claim that newer releases can run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GenerationMinecraftVersion {
    /// Java Edition 1.21.8, the currently pinned `FerrumC` release.
    V1_21_8,
}

impl GenerationMinecraftVersion {
    /// Returns the major, minor and patch release numbers.
    #[must_use]
    pub const fn components(self) -> [u16; 3] {
        match self {
            Self::V1_21_8 => [1, 21, 8],
        }
    }
}

/// Generator algorithm identified independently of its envelope schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GeneratorKind {
    /// `FerrumC`'s existing fixed-layer flat profile, not Vanilla flat settings.
    FerrumcFlat,
    /// Cold Overworld terrain preview with explicitly versioned decoration subsets.
    FerrumcNaturalPreview,
}

impl GeneratorKind {
    const fn code(self) -> u16 {
        match self {
            Self::FerrumcFlat => 1,
            Self::FerrumcNaturalPreview => 2,
        }
    }
}

/// Dimension geometry and rules supported by a generation identity.
///
/// A dimension kind does not replace the world's runtime/storage dimension ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GenerationDimension {
    /// Overworld geometry: minimum Y -64, height 384.
    Overworld,
}

impl GenerationDimension {
    const fn code(self) -> u8 {
        match self {
            Self::Overworld => 0,
        }
    }
}

/// Complete supported identity of a reproducible generated chunk baseline.
///
/// Supports Minecraft 1.21.8 / Overworld, with distinct flat revision 1 and
/// natural terrain preview revisions 1 through 5. Decode rejects other metadata
/// explicitly. Seeds are retained even when an algorithm is seed-independent.
///
/// This is a value contract. It does not persist the identity or guard the
/// storage/load path; consumers must explicitly adopt its compatibility gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GenerationIdentity {
    minecraft_version: GenerationMinecraftVersion,
    generator_kind: GeneratorKind,
    generator_revision: u32,
    seed: WorldSeed,
    dimension: GenerationDimension,
}

impl GenerationIdentity {
    /// Selects the existing `FerrumC` flat revision-1 Overworld baseline.
    #[must_use]
    pub const fn flat_overworld(seed: WorldSeed) -> Self {
        Self {
            minecraft_version: GenerationMinecraftVersion::V1_21_8,
            generator_kind: GeneratorKind::FerrumcFlat,
            generator_revision: FLAT_REVISION,
            seed,
            dimension: GenerationDimension::Overworld,
        }
    }

    /// Selects real cold Overworld terrain through the accepted carver stage.
    ///
    /// Revision 1 omits placed features, trees, structures and fluid simulation.
    /// It is a terrain preview, not a complete Survival world generator.
    #[must_use]
    pub const fn natural_overworld(seed: WorldSeed) -> Self {
        Self {
            minecraft_version: GenerationMinecraftVersion::V1_21_8,
            generator_kind: GeneratorKind::FerrumcNaturalPreview,
            generator_revision: NATURAL_PREVIEW_REVISION,
            seed,
            dimension: GenerationDimension::Overworld,
        }
    }

    /// Selects the separately versioned original short-grass subset preview.
    ///
    /// Revision 2 runs plains/forest patches against isolated cold source chunks;
    /// it omits other FEATURES and never reinterprets revision-1 saved terrain.
    #[must_use]
    pub const fn natural_overworld_with_grass(seed: WorldSeed) -> Self {
        let mut identity = Self::natural_overworld(seed);
        identity.generator_revision = 2;
        identity
    }

    /// Selects separately versioned original oak/birch shapes and short grass.
    ///
    /// Revision 3 has an explicit isolated-source selector/decorator subset policy.
    #[must_use]
    pub const fn natural_overworld_with_trees(seed: WorldSeed) -> Self {
        let mut identity = Self::natural_overworld(seed);
        identity.generator_revision = 3;
        identity
    }
    /// Selects revision 4: trees/grass and an isolated-source placed ore subset.
    ///
    /// Existing identities never select this revision implicitly. Full FEATURES,
    /// structures and Vanilla seed parity remain outside this Survival preview.
    #[must_use]
    pub const fn natural_overworld_survival(seed: WorldSeed) -> Self {
        let mut identity = Self::natural_overworld(seed);
        identity.generator_revision = 4;
        identity
    }
    /// Selects revision 5 with additional biome vegetation and bounded structures.
    ///
    /// Existing saved baselines remain on their exact previous revision. This
    /// native playable profile does not declare full Vanilla feature parity.
    #[must_use]
    pub const fn natural_overworld_playable(seed: WorldSeed) -> Self {
        let mut identity = Self::natural_overworld(seed);
        identity.generator_revision = 5;
        identity
    }

    /// Returns the Minecraft release whose rules this baseline uses.
    #[must_use]
    pub const fn minecraft_version(self) -> GenerationMinecraftVersion {
        self.minecraft_version
    }

    /// Returns the generator algorithm, independently of its revision.
    #[must_use]
    pub const fn generator_kind(self) -> GeneratorKind {
        self.generator_kind
    }

    /// Returns the algorithm revision, independently of the envelope schema.
    #[must_use]
    pub const fn generator_revision(self) -> u32 {
        self.generator_revision
    }

    /// Returns the selected numeric world seed.
    #[must_use]
    pub const fn seed(self) -> WorldSeed {
        self.seed
    }

    /// Returns the supported dimension geometry and rules.
    #[must_use]
    pub const fn dimension(self) -> GenerationDimension {
        self.dimension
    }

    /// Encodes this identity into the fixed, allocation-free schema-1 envelope.
    ///
    /// Layout: `FCGI`, big-endian `u16` schema, three big-endian `u16` Minecraft
    /// release components, `u16` algorithm, `u32` generator revision, `i64` seed,
    /// and `u8` dimension. Every multibyte number is big-endian. Algorithm 1 is
    /// `FerrumC` flat, 2 is the natural terrain preview; dimension 0 is Overworld. This is `FerrumC` metadata, not a
    /// Minecraft packet or Vanilla save record.
    #[must_use]
    pub fn encode(self) -> [u8; GENERATION_IDENTITY_LEN] {
        let mut bytes = [0; GENERATION_IDENTITY_LEN];
        bytes[..4].copy_from_slice(&IDENTITY_MAGIC);
        bytes[4..6].copy_from_slice(&IDENTITY_SCHEMA.to_be_bytes());
        let [major, minor, patch] = self.minecraft_version.components();
        bytes[6..8].copy_from_slice(&major.to_be_bytes());
        bytes[8..10].copy_from_slice(&minor.to_be_bytes());
        bytes[10..12].copy_from_slice(&patch.to_be_bytes());
        bytes[12..14].copy_from_slice(&self.generator_kind.code().to_be_bytes());
        bytes[14..18].copy_from_slice(&self.generator_revision.to_be_bytes());
        bytes[18..26].copy_from_slice(&self.seed.value().to_be_bytes());
        bytes[26] = self.dimension.code();
        bytes
    }

    /// Decodes exactly one supported identity, rejecting all trailing bytes.
    ///
    /// Length is checked before reading any field. Unknown schemas, releases,
    /// algorithms, algorithm revisions and dimensions have distinct errors;
    /// none fall back to the current flat generator.
    pub fn decode(bytes: &[u8]) -> Result<Self, GenerationError> {
        let bytes: &[u8; GENERATION_IDENTITY_LEN] =
            bytes
                .try_into()
                .map_err(|_| GenerationError::InvalidLength {
                    actual: bytes.len(),
                    expected: GENERATION_IDENTITY_LEN,
                })?;
        let magic = [bytes[0], bytes[1], bytes[2], bytes[3]];
        if magic != IDENTITY_MAGIC {
            return Err(GenerationError::InvalidMagic { actual: magic });
        }
        let schema = u16::from_be_bytes([bytes[4], bytes[5]]);
        if schema != IDENTITY_SCHEMA {
            return Err(GenerationError::UnsupportedSchema { schema });
        }
        let major = u16::from_be_bytes([bytes[6], bytes[7]]);
        let minor = u16::from_be_bytes([bytes[8], bytes[9]]);
        let patch = u16::from_be_bytes([bytes[10], bytes[11]]);
        if [major, minor, patch] != GenerationMinecraftVersion::V1_21_8.components() {
            return Err(GenerationError::UnsupportedMinecraftVersion {
                major,
                minor,
                patch,
            });
        }
        let kind = u16::from_be_bytes([bytes[12], bytes[13]]);
        let (generator_kind, supported_revision) = match kind {
            1 => (GeneratorKind::FerrumcFlat, FLAT_REVISION),
            2 => (
                GeneratorKind::FerrumcNaturalPreview,
                NATURAL_PREVIEW_REVISION,
            ),
            _ => return Err(GenerationError::UnknownGenerator { kind }),
        };
        let revision = u32::from_be_bytes([bytes[14], bytes[15], bytes[16], bytes[17]]);
        if revision != supported_revision
            && !(generator_kind == GeneratorKind::FerrumcNaturalPreview
                && matches!(revision, 2..=5))
        {
            return Err(GenerationError::UnsupportedGeneratorRevision {
                kind: generator_kind,
                revision,
            });
        }
        let seed = i64::from_be_bytes([
            bytes[18], bytes[19], bytes[20], bytes[21], bytes[22], bytes[23], bytes[24], bytes[25],
        ]);
        if bytes[26] != GenerationDimension::Overworld.code() {
            return Err(GenerationError::UnsupportedDimension {
                dimension: bytes[26],
            });
        }
        Ok(match generator_kind {
            GeneratorKind::FerrumcFlat => Self::flat_overworld(WorldSeed::new(seed)),
            GeneratorKind::FerrumcNaturalPreview if revision == 5 => {
                Self::natural_overworld_playable(WorldSeed::new(seed))
            }
            GeneratorKind::FerrumcNaturalPreview if revision == 4 => {
                Self::natural_overworld_survival(WorldSeed::new(seed))
            }
            GeneratorKind::FerrumcNaturalPreview if revision == 3 => {
                Self::natural_overworld_with_trees(WorldSeed::new(seed))
            }
            GeneratorKind::FerrumcNaturalPreview if revision == 2 => {
                Self::natural_overworld_with_grass(WorldSeed::new(seed))
            }
            GeneratorKind::FerrumcNaturalPreview => Self::natural_overworld(WorldSeed::new(seed)),
        })
    }

    /// Requires an exact match with a stored or requested supported identity.
    ///
    /// The full identity participates in equality, including algorithm and seed.
    /// Unknown components are rejected earlier by [`Self::decode`].
    pub fn ensure_compatible(self, actual: Self) -> Result<(), GenerationError> {
        if self == actual {
            Ok(())
        } else {
            Err(GenerationError::IncompatibleIdentity {
                expected: self,
                actual,
            })
        }
    }
}

/// Pure, synchronous generation of a versioned deterministic chunk baseline.
///
/// Implementations must return the same identity throughout their lifetime and
/// produce the same blocks, palette layout and dirty masks for a given identity
/// and [`ChunkPos`], independent of request order or previous calls. The trait
/// performs no I/O and owns no storage or network handles. CPU work remains
/// synchronous so wiring can put it on the appropriate bounded worker lane.
pub trait ChunkGenerator: Send + Sync {
    /// Returns the algorithm, revision, release, seed and dimension contract.
    fn identity(&self) -> GenerationIdentity;

    /// Produces the deterministic baseline for the requested chunk column.
    fn generate_chunk(&self, pos: ChunkPos) -> Chunk;
}

/// Pure generation whose producer refusals propagate without substituting terrain.
///
/// Implementations retain a stable identity and deterministically return the same
/// complete chunk or classified error for a position, independent of request order.
/// Callers must place synchronous CPU work on a bounded generation worker lane.
pub trait FallibleChunkGenerator: Send + Sync {
    /// Returns the complete versioned baseline identity.
    fn identity(&self) -> GenerationIdentity;

    /// Produces a complete baseline, or exposes the refusing producer's error.
    fn try_generate_chunk(&self, pos: ChunkPos) -> Result<Chunk, WorldError>;
}

impl FallibleChunkGenerator for crate::FlatWorldGenerator {
    fn identity(&self) -> GenerationIdentity {
        self.identity()
    }

    fn try_generate_chunk(&self, pos: ChunkPos) -> Result<Chunk, WorldError> {
        Ok(self.generate(pos))
    }
}

/// Classified failures of identity decoding or baseline compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum GenerationError {
    /// The input was truncated or contained trailing/oversized data.
    #[error("generation identity length {actual} does not equal {expected}")]
    InvalidLength {
        /// Actual input byte length.
        actual: usize,
        /// Required fixed envelope length.
        expected: usize,
    },
    /// The fixed-length input had a different record discriminator.
    #[error("invalid generation identity magic {actual:?}")]
    InvalidMagic {
        /// The input's first four bytes.
        actual: [u8; 4],
    },
    /// The metadata envelope schema is not supported.
    #[error("unsupported generation identity envelope schema {schema}")]
    UnsupportedSchema {
        /// Rejected schema, independently of generator revision.
        schema: u16,
    },
    /// The identity belongs to a different Minecraft release.
    #[error("unsupported generation Minecraft version {major}.{minor}.{patch}")]
    UnsupportedMinecraftVersion {
        /// Rejected major release number.
        major: u16,
        /// Rejected minor release number.
        minor: u16,
        /// Rejected patch release number.
        patch: u16,
    },
    /// The identity selects an algorithm that `FerrumC` does not implement.
    #[error("unknown generation algorithm {kind}")]
    UnknownGenerator {
        /// Rejected algorithm discriminator.
        kind: u16,
    },
    /// A known algorithm's revision is not supported.
    #[error("unsupported generation algorithm {kind:?} revision {revision}")]
    UnsupportedGeneratorRevision {
        /// Known algorithm whose revision was rejected.
        kind: GeneratorKind,
        /// Rejected algorithm revision.
        revision: u32,
    },
    /// The identity selects unsupported dimension geometry or rules.
    #[error("unsupported generation dimension {dimension}")]
    UnsupportedDimension {
        /// Rejected dimension discriminator.
        dimension: u8,
    },
    /// Two supported identities select different generated baselines.
    #[error("incompatible generation identity: expected {expected:?}, actual {actual:?}")]
    IncompatibleIdentity {
        /// Required identity, typically read from durable world metadata.
        expected: GenerationIdentity,
        /// Identity proposed by the caller.
        actual: GenerationIdentity,
    },
}
