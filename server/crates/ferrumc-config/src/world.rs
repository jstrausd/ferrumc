//! World-content configuration: where the world's initial terrain comes from.
//!
//! [`WorldConfig`] is the declarative `[world]` TOML table. Numeric seed and
//! supported preset choices remain optional declarations: the application
//! reconciles them with an existing persisted identity or explicitly chooses
//! defaults for a fresh world. This type never selects a seed or performs I/O.
//! Pointing `anvil_import_dir` at a vanilla Anvil `region/` directory imports
//! that prebuilt map at startup; generation settings independently describe
//! the fallback for chunks not supplied by the import. The import is not a
//! reconstruction of Vanilla's original seed or generator settings.
//!
//! This is distinct from the application's top-level `world_dir` key, which names
//! where the durable world *database* is persisted: `world_dir` is the save
//! location, `[world].anvil_import_dir` is the one-time initial *content* source.

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// A supported `FerrumC` generator profile, independent of the numeric seed.
///
/// The only accepted TOML spelling is `"ferrumc_flat"`: the implemented fixed
/// flat profile, not Vanilla superflat settings or a claim of noise generation.
/// Unknown or unimplemented preset names fail deserialization. This declarative
/// type does not depend on world/storage/simulation or construct a generator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[non_exhaustive]
pub enum WorldGenerationPreset {
    /// `FerrumC`'s implemented fixed-layer flat Overworld profile.
    #[serde(rename = "ferrumc_flat")]
    FerrumcFlat,
}

/// Declarative world-content configuration, deserialized from the `[world]` TOML
/// table.
///
/// Every field is optional. Omitted seed/preset values remain `None`, allowing
/// the application to reuse the stored generation identity rather than invent
/// a new baseline. Fresh-world defaults are an explicit application decision.
/// A supplied seed must be a TOML integer in the signed 64-bit range; textual,
/// random, floating-point and out-of-range seeds are refused. Unknown keys and
/// unimplemented presets are rejected so typos never silently change the world.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WorldConfig {
    /// Optional path to a vanilla Anvil `region/` directory whose `r.<x>.<z>.mca`
    /// region files are imported into the world store at startup.
    ///
    /// `None` (the default — the key omitted) declares no import. `Some(dir)`
    /// imports every region file found in `dir` *before*
    /// the server accepts connections, so a prebuilt map populates the world. A
    /// missing directory, a directory with no region files, or a malformed region
    /// file aborts startup with a clear error rather than silently serving flat
    /// terrain.
    pub anvil_import_dir: Option<PathBuf>,
    seed: Option<i64>,
    preset: Option<WorldGenerationPreset>,
}

impl WorldConfig {
    /// Returns the configured Anvil import directory, or `None` when the server
    /// has no initial import declaration. Independent seed/preset choices still
    /// apply to generation outside the imported chunks.
    #[must_use]
    pub fn anvil_import_dir(&self) -> Option<&Path> {
        self.anvil_import_dir.as_deref()
    }

    /// Returns the explicitly selected numeric seed without choosing a default.
    ///
    /// `Some(0)` is an explicit seed and differs from `None`. On restart the
    /// application can resolve omission against the persisted identity; this
    /// accessor never reads storage or generates/parses a random/text seed.
    #[must_use]
    pub const fn seed(&self) -> Option<i64> {
        self.seed
    }

    /// Returns the explicitly selected supported profile, retaining omission.
    ///
    /// The application resolves `None` against persisted metadata or its
    /// documented fresh-world default; the config layer performs no fallback.
    #[must_use]
    pub const fn preset(&self) -> Option<WorldGenerationPreset> {
        self.preset
    }

    /// Replaces generation declarations while preserving the import directory.
    ///
    /// The typed inputs are bounded by `i64` and the supported preset enum;
    /// no free-form preset or textual/random seed is accepted. `None` remains
    /// omitted and can clear a prior declaration. This constructs config only,
    /// not a persisted identity or a validated binding to an existing world.
    #[must_use]
    pub fn with_generation(
        mut self,
        seed: Option<i64>,
        preset: Option<WorldGenerationPreset>,
    ) -> Self {
        self.seed = seed;
        self.preset = preset;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_imports_nothing() {
        let config = WorldConfig::default();
        assert_eq!(config.anvil_import_dir(), None);
    }

    #[test]
    fn an_omitted_table_keeps_the_defaults() {
        let parsed: WorldConfig = toml::from_str("").expect("empty table is valid");
        assert_eq!(parsed, WorldConfig::default());
    }

    #[test]
    fn anvil_import_dir_parses_from_toml() {
        let parsed: WorldConfig =
            toml::from_str(r#"anvil_import_dir = "/srv/maps/spawn/region""#).expect("valid table");
        assert_eq!(
            parsed.anvil_import_dir(),
            Some(Path::new("/srv/maps/spawn/region"))
        );
    }

    #[test]
    fn unknown_field_is_rejected() {
        assert!(toml::from_str::<WorldConfig>("bogus = 1").is_err());
    }
}
