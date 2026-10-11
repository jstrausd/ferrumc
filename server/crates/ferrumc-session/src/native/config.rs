//! Positive collector-session limits, the validated registry layout, and the
//! sealed per-router session key.

use std::num::{NonZeroU64, NonZeroUsize};
use std::sync::Weak;

use ferrumc_core::PlayerId;
use ferrumc_proto::generated::configuration::{KnownPack, RegistryData};
use ferrumc_sim::GenerationBinding;

use super::CollectorSessionError;

/// Hard maximum of installed sessions plus provisional reservations.
pub const MAX_COLLECTOR_SESSIONS: usize = 1024;
/// Hard maximum of player baselines one viewer may hold.
pub const MAX_BASELINES_PER_VIEWER: usize = 1023;
/// Hard maximum of player baselines across all viewers.
pub const MAX_TOTAL_BASELINES: usize = 262_144;
/// Hard maximum of one collector audience (observers plus self).
pub const MAX_COLLECTOR_AUDIENCE: usize = 1024;
/// Hard maximum of readonly entry visits admitted for one scan.
pub const MAX_COLLECTOR_WORK: usize = 4_194_304;
/// Hard maximum of checked scratch bytes for one plan.
pub const MAX_COLLECTOR_SCRATCH: usize = 1_048_576;
/// Hard maximum of the bounded native lane, equal to the router's default
/// shard-input capacity.
pub const MAX_NATIVE_LANE_CAPACITY: usize = crate::DEFAULT_SHARD_INPUT_CAPACITY;
/// Largest configured simulation distance accepted for the native layout.
pub const MAX_SIMULATION_DISTANCE: i32 = 32;

/// Explicit positive collector-session limits, validated before any session.
///
/// Every value is a nonzero operator agreement with a hard maximum; nothing is
/// clamped. These are chosen bounds, not measurements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollectorSessionLimits {
    sessions: NonZeroUsize,
    baselines_per_viewer: NonZeroUsize,
    total_baselines: NonZeroUsize,
    audience: NonZeroUsize,
    work: NonZeroUsize,
    scratch: NonZeroUsize,
}

impl CollectorSessionLimits {
    /// Validates the six positive limits against their hard maxima and the
    /// cross rules `baselines_per_viewer < sessions` and `audience <= sessions`.
    ///
    /// # Errors
    ///
    /// Returns [`CollectorSessionError::InvalidLimit`] naming the first
    /// violated rule.
    pub fn new(
        sessions: NonZeroUsize,
        baselines_per_viewer: NonZeroUsize,
        total_baselines: NonZeroUsize,
        audience: NonZeroUsize,
        work: NonZeroUsize,
        scratch: NonZeroUsize,
    ) -> Result<Self, CollectorSessionError> {
        for (value, max, name) in [
            (sessions.get(), MAX_COLLECTOR_SESSIONS, "sessions"),
            (
                baselines_per_viewer.get(),
                MAX_BASELINES_PER_VIEWER,
                "baselines per viewer",
            ),
            (
                total_baselines.get(),
                MAX_TOTAL_BASELINES,
                "total baselines",
            ),
            (audience.get(), MAX_COLLECTOR_AUDIENCE, "audience"),
            (work.get(), MAX_COLLECTOR_WORK, "work"),
            (scratch.get(), MAX_COLLECTOR_SCRATCH, "scratch"),
        ] {
            if value > max {
                return Err(CollectorSessionError::InvalidLimit(name));
            }
        }
        if baselines_per_viewer >= sessions {
            return Err(CollectorSessionError::InvalidLimit(
                "baselines per viewer must be below sessions",
            ));
        }
        if audience > sessions {
            return Err(CollectorSessionError::InvalidLimit(
                "audience must not exceed sessions",
            ));
        }
        Ok(Self {
            sessions,
            baselines_per_viewer,
            total_baselines,
            audience,
            work,
            scratch,
        })
    }

    /// Installed sessions (legacy and native, including pending leaves) plus
    /// provisional native reservations.
    pub const fn sessions(self) -> NonZeroUsize {
        self.sessions
    }

    /// Player baselines one viewer may hold.
    pub const fn baselines_per_viewer(self) -> NonZeroUsize {
        self.baselines_per_viewer
    }

    /// Player baselines across all viewers.
    pub const fn total_baselines(self) -> NonZeroUsize {
        self.total_baselines
    }

    /// Entries of one collector audience.
    pub const fn audience(self) -> NonZeroUsize {
        self.audience
    }

    /// Readonly entry visits admitted for one scan.
    pub const fn work(self) -> NonZeroUsize {
        self.work
    }

    /// Checked scratch bytes for one plan.
    pub const fn scratch(self) -> NonZeroUsize {
        self.scratch
    }
}

/// The one supported native registry/generation layout.
///
/// It is configuration-data validation of the exact app registry producer, not
/// a configuration or socket receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeRegistryLayout {
    /// Minecraft 1.21.8 core pack with exactly `minecraft:overworld` at
    /// dimension-type index 0 and the `FerrumC` flat revision-1 generator.
    FlatOverworld1_21_8,
}

/// Dimension-type registry identifier of the supported layout.
pub(crate) const DIMENSION_TYPE_REGISTRY: &str = "minecraft:dimension_type";
/// The single dimension of the supported layout.
pub(crate) const OVERWORLD: &str = "minecraft:overworld";
/// Dimension-type index of [`OVERWORLD`] in the supported layout.
pub(crate) const OVERWORLD_DIMENSION_TYPE: i32 = 0;

impl NativeRegistryLayout {
    /// Validates the dimension registry and core pack the connection advertises.
    pub(crate) fn validate(
        dimension_registry: &RegistryData,
        core_pack: &KnownPack,
    ) -> Result<Self, CollectorSessionError> {
        if dimension_registry.registry_id().as_str() != DIMENSION_TYPE_REGISTRY {
            return Err(CollectorSessionError::UnsupportedRegistryLayout(
                "registry identifier",
            ));
        }
        let [entry] = dimension_registry.entries() else {
            return Err(CollectorSessionError::UnsupportedRegistryLayout(
                "dimension entry count",
            ));
        };
        if entry.entry_id().as_str() != OVERWORLD {
            return Err(CollectorSessionError::UnsupportedRegistryLayout(
                "dimension entry name",
            ));
        }
        if entry.data().is_some() {
            return Err(CollectorSessionError::UnsupportedRegistryLayout(
                "dimension entry data",
            ));
        }
        if core_pack.namespace().as_str() != "minecraft"
            || core_pack.id().as_str() != "core"
            || core_pack.version().as_str() != "1.21.8"
        {
            return Err(CollectorSessionError::UnsupportedRegistryLayout(
                "core known pack",
            ));
        }
        Ok(Self::FlatOverworld1_21_8)
    }

    /// Checks that `generation` is the scope this layout describes: world 0,
    /// dimension 0, Minecraft 1.21.8, `FerrumC` flat revision 1, Overworld.
    ///
    /// The generator kind and dimension geometry are read from the documented
    /// fixed schema-1 identity envelope, so this crate needs no world-model
    /// dependency to name them.
    pub(crate) fn supports(self, generation: GenerationBinding) -> bool {
        match self {
            Self::FlatOverworld1_21_8 => {
                let key = generation.key();
                let identity = generation.identity();
                let encoded = identity.encode();
                // Layout: magic, schema 1, release 1.21.8, algorithm 1 (flat),
                // revision 1; byte 26 is the dimension kind (0 = Overworld).
                let fixed: [u8; 18] = [
                    b'F', b'C', b'G', b'I', 0, 1, 0, 1, 0, 21, 0, 8, 0, 1, 0, 0, 0, 1,
                ];
                key.world().get() == 0
                    && key.dimension().get() == 0
                    && identity.minecraft_version().components() == [1, 21, 8]
                    && identity.generator_revision() == 1
                    && encoded.get(..18) == Some(&fixed[..])
                    && encoded.get(26) == Some(&0)
            }
        }
    }
}

/// The router's private identity; keys compare against it with `Weak::ptr_eq`.
#[derive(Debug)]
pub(crate) struct RouterIdentity;

/// Sealed routing key of one native session incarnation.
///
/// It holds only a weak router identity, the player and the router-local
/// session incarnation. It confers no authority: it cannot build an input,
/// reach a shard, or rebind a session. Keys of another router, of an earlier
/// incarnation of the same UUID, or of a removed session never address the
/// current session.
///
/// No public constructor or field exists:
///
/// ```compile_fail
/// use ferrumc_session::PlayerSessionKey;
/// fn forge(player: ferrumc_core::PlayerId) -> PlayerSessionKey {
///     PlayerSessionKey { player }
/// }
/// ```
#[derive(Clone)]
pub struct PlayerSessionKey {
    pub(crate) router: Weak<RouterIdentity>,
    pub(crate) player: PlayerId,
    pub(crate) incarnation: NonZeroU64,
}

impl PlayerSessionKey {
    /// The player this session belongs to.
    pub const fn player(&self) -> PlayerId {
        self.player
    }
}

impl PartialEq for PlayerSessionKey {
    fn eq(&self, other: &Self) -> bool {
        Weak::ptr_eq(&self.router, &other.router)
            && self.player == other.player
            && self.incarnation == other.incarnation
    }
}

impl Eq for PlayerSessionKey {}

impl std::fmt::Debug for PlayerSessionKey {
    // The weak router address is identity, never data: it is not printed.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlayerSessionKey")
            .field("player", &self.player)
            .field("incarnation", &self.incarnation)
            .finish_non_exhaustive()
    }
}
