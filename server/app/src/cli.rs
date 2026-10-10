//! Command-line interface and config bootstrap for the `ferrumc` binary.
//!
//! This module owns the [`clap`]-derived [`Cli`] argument parser, the rules for
//! resolving which config file to read, and the first-run logic that writes a
//! commented default config and then continues with defaults rather than dying.
//!
//! It lives in the library (not `main.rs`) so the integration tests in
//! `app/tests/` — which link against the lib only — can exercise argument
//! parsing, override precedence, and the first-run/template behaviour directly,
//! without spawning the binary.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use clap::{Args, Parser, Subcommand};
use ferrumc_core::{DimensionId, WorldId};
use ferrumc_storage::{
    GenerationKey, GenerationMigrationPreparation, GenerationMigrationReport,
    GenerationMigrationTable, LegacyGenerationDeclaration, RedbStore,
};
use std::fmt::Write as _;

use crate::config::AppConfig;

/// Config file resolved against the current working directory when neither
/// `--config` nor `$FERRUMC_CONFIG` is set.
const DEFAULT_CONFIG_FILE: &str = "config.toml";

/// World directory the shipping server persists to when the config names none.
///
/// This is what makes the durable redb store the runtime default. Tests construct
/// [`AppConfig`] directly and leave `world_dir` unset (in-memory) or point it at a
/// temp directory, so they never touch this.
const DEFAULT_WORLD_DIR: &str = "world";

/// The commented default config written verbatim on first run. Authored in
/// `app/config.template.toml`; every key is commented out, so the file parses to
/// the documented defaults until an operator edits it.
const CONFIG_TEMPLATE: &str = include_str!("../config.template.toml");

/// Parsed command-line arguments for the `ferrumc` server binary.
///
/// `clap` provides `--help` and `--version` automatically; the remaining flags
/// select the config file and optionally override the listen address. Invalid
/// values (a non-numeric `--port`, an unparseable `--bind`, an unknown flag) are
/// rejected by `clap` with a clear usage error.
#[derive(Debug, Parser)]
#[command(
    name = "ferrumc",
    version,
    about = "FerrumC: a Minecraft Java 1.21.8 (protocol 772) server",
    long_about = "FerrumC: a high-performance Minecraft Java 1.21.8 (protocol 772) server.\n\n\
                  Runs in offline mode by default. With no config file present, a commented \
                  config.toml is written next to it on first run and the server starts with \
                  defaults — edit it and restart to customise. Fresh worlds explicitly choose seed0 and ferrumc_flat; omitted settings reuse the stored identity on restart. Legacy format3 requires the explicit offline migrate-generation command. Set RUST_LOG=debug for verbose logs."
)]
pub struct Cli {
    /// Explicit offline operations, dispatched before config bootstrap or networking.
    #[command(subcommand)]
    command: Option<OfflineCommand>,
    /// Path to the TOML config file. Falls back to `$FERRUMC_CONFIG`, then
    /// `./config.toml`. Written with commented defaults on first run if missing.
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,
    /// Override the listen port, replacing the port of the configured bind
    /// address. Applied after `--bind`, so it wins if both are given.
    #[arg(long, value_name = "PORT")]
    pub port: Option<u16>,
    /// Override the full bind address, e.g. `0.0.0.0:25565`. `--port` still
    /// refines the port afterwards if also supplied.
    #[arg(long, value_name = "ADDR")]
    pub bind: Option<SocketAddr>,
}

#[derive(Debug, Clone, Subcommand)]
enum OfflineCommand {
    /// Validate and explicitly migrate legacy `FerrumC` format 3 to format 4.
    MigrateGeneration(GenerationMigrationArgs),
}

#[derive(Debug, Clone, Args)]
struct GenerationMigrationArgs {
    /// Existing closed source world.redb; missing files are never created.
    #[arg(long, value_name = "PATH")]
    source: PathBuf,
    /// New exclusive backup destination, or a hash-bound existing backup.
    #[arg(long, value_name = "PATH", required_unless_present = "prepare_only")]
    backup: Option<PathBuf>,
    /// Complete legacy world:dimension scopes (numeric u32:u32), repeatable.
    #[arg(long, value_name = "WORLD:DIMENSION", required = true, value_parser = parse_scope)]
    scope: Vec<GenerationKey>,
    /// Explicitly assigns Minecraft1.21.8 / `FerrumC` flat-v1 / Overworld / seed0.
    #[arg(long, required = true)]
    declare_legacy_flat_v1_seed_zero: bool,
    /// Validate/report only; does not create a backup or migrate.
    #[arg(long, conflicts_with_all = ["backup", "backup_sha256"])]
    prepare_only: bool,
    /// SHA256 recorded after the original backup close, required for reuse.
    #[arg(long, value_name = "HEX", requires = "backup", value_parser = parse_sha256)]
    backup_sha256: Option<[u8; 32]>,
}

fn parse_scope(value: &str) -> Result<GenerationKey, String> {
    let (world, dimension) = value
        .split_once(':')
        .ok_or_else(|| "scope must be WORLD:DIMENSION".to_owned())?;
    let world = world
        .parse::<u32>()
        .map_err(|_| "world must be an unsigned u32".to_owned())?;
    let dimension = dimension
        .parse::<u32>()
        .map_err(|_| "dimension must be an unsigned u32".to_owned())?;
    Ok(GenerationKey::new(
        WorldId::new(world),
        DimensionId::new(dimension),
    ))
}

fn parse_sha256(value: &str) -> Result<[u8; 32], String> {
    if value.len() != 64 || !value.is_ascii() {
        return Err("SHA256 must contain exactly 64 hexadecimal ASCII digits".to_owned());
    }
    let mut result = [0; 32];
    for (index, byte) in result.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| "SHA256 contains a non-hexadecimal digit".to_owned())?;
    }
    Ok(result)
}

fn hex_sha256(bytes: &[u8; 32]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(64), |mut output, byte| {
            let _ = write!(output, "{byte:02x}");
            output
        })
}

fn migration_report(report: &GenerationMigrationReport) -> String {
    let mut output = format!(
        "source={} format={} target={} file_bytes={} logical_bytes={} snapshot_sha256={}\nlegacy declaration: Minecraft1.21.8 FerrumC-flat-v1 Overworld seed0 (explicit assignment)\n",
        report.source_path().display(), report.source_format(), report.target_format(),
        report.source_file_len(), report.logical_bytes(), hex_sha256(report.snapshot_sha256()),
    );
    for scope in report.declaration().scopes() {
        let _ = writeln!(
            output,
            "scope={}:{}",
            scope.world().get(),
            scope.dimension().get()
        );
    }
    for table in [
        GenerationMigrationTable::Metadata,
        GenerationMigrationTable::Chunks,
        GenerationMigrationTable::ChunkOverlays,
        GenerationMigrationTable::Entities,
        GenerationMigrationTable::Players,
        GenerationMigrationTable::PluginValues,
        GenerationMigrationTable::JournalRecords,
        GenerationMigrationTable::JournalReceipts,
        GenerationMigrationTable::GenerationIdentities,
    ] {
        let _ = writeln!(output, "{table:?}={}", report.records(table));
    }
    output
}

impl GenerationMigrationArgs {
    fn run(self) -> anyhow::Result<String> {
        let declaration = LegacyGenerationDeclaration::ferrumc_flat_v1_seed_zero(self.scope)?;
        let preparation = RedbStore::prepare_generation_migration(&self.source, declaration)?;
        match preparation {
            GenerationMigrationPreparation::AlreadyCompatible(report) => Ok(format!(
                "{}already compatible: complete format4 validated; no migration needed\n",
                migration_report(&report)
            )),
            GenerationMigrationPreparation::Required(prepared) => {
                let mut output = migration_report(prepared.report());
                if self.prepare_only {
                    output.push_str("prepare-only: validated; source remains format3\n");
                    return Ok(output);
                }
                let backup_path = self
                    .backup
                    .ok_or_else(|| anyhow::anyhow!("migration requires an explicit backup path"))?;
                let backed_up = match self.backup_sha256 {
                    Some(expected) => prepared.verify_existing_backup(&backup_path, expected)?,
                    None => prepared.create_backup(&backup_path)?,
                };
                let backup = backed_up.backup();
                let _ = writeln!(
                    output,
                    "verified backup={} file_bytes={} file_sha256={} snapshot_sha256={}",
                    backup.path().display(),
                    backup.file_len(),
                    hex_sha256(backup.file_sha256()),
                    hex_sha256(backup.snapshot_sha256())
                );
                let outcome = backed_up.migrate().map_err(|error| {
                    // If commit fails after a new backup was created, preserve
                    // its verified hash in stderr so an explicit retry can use
                    // --backup-sha256 rather than guessing or overwriting it.
                    anyhow::anyhow!("{output}migration failed after verified backup: {error}")
                })?;
                let _ = writeln!(
                    output,
                    "migration committed: format{}; backup_sha256={}",
                    outcome.report().target_format(),
                    hex_sha256(outcome.backup().file_sha256())
                );
                Ok(output)
            }
        }
    }
}

impl Cli {
    /// Runs a requested offline operation on a blocking worker, without reading
    /// or writing normal server configuration and without binding networking.
    /// Returns `None` for normal server startup.
    pub async fn run_offline(&self) -> anyhow::Result<Option<String>> {
        let Some(OfflineCommand::MigrateGeneration(args)) = self.command.clone() else {
            return Ok(None);
        };
        tokio::task::spawn_blocking(move || args.run())
            .await
            .map_err(|error| anyhow::anyhow!("offline migration task failed: {error}"))?
            .map(Some)
    }

    /// Resolves which config file to use.
    ///
    /// Precedence: the `--config` flag, then the `FERRUMC_CONFIG` environment
    /// variable, then `./config.toml` relative to the current directory.
    #[must_use]
    pub fn config_path(&self) -> PathBuf {
        self.config
            .clone()
            .or_else(|| std::env::var_os("FERRUMC_CONFIG").map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_FILE))
    }
}

/// Loads the config at `path`, writing a commented default on first run, then
/// applies the CLI overrides and fills the runtime defaults.
///
/// Behaviour:
/// - **Missing file (first run):** writes [`CONFIG_TEMPLATE`] to `path`, logs a
///   clear message, and continues with [`AppConfig::default`]. The server never
///   dies just because no config exists yet. Using the defaults directly (rather
///   than re-parsing the file just written) guarantees first run cannot fail even
///   if the shipped template had a bug; the all-commented template parses to the
///   same defaults on the next run, so both starts converge.
/// - **Existing file:** read and parsed; a malformed document is the one allowed
///   startup-config error path — it returns `Err`, and `main` exits non-zero with
///   a message naming the offending key and the expected type (from serde/toml).
/// - **Overrides:** `bind` is applied first (full address), then `port` refines
///   the port, so an explicit `--port` always wins.
/// - **Runtime default:** an unset `world_dir` is filled with the shipping
///   default world directory so a real server persists by default.
///
/// # Errors
///
/// Returns an error if the file exists but cannot be read, if an existing file is
/// a malformed or invalid config, or if the template cannot be written to a
/// missing `path` (for example, an unwritable directory).
pub fn load_or_init_config(
    path: &Path,
    port: Option<u16>,
    bind: Option<SocketAddr>,
) -> anyhow::Result<AppConfig> {
    let config = if path.exists() {
        let text = std::fs::read_to_string(path)
            .map_err(|err| anyhow::anyhow!("reading config {}: {err}", path.display()))?;
        AppConfig::from_toml_str(&text)
            .map_err(|err| anyhow::anyhow!("invalid config {}: {err}", path.display()))?
    } else {
        std::fs::write(path, CONFIG_TEMPLATE).map_err(|err| {
            anyhow::anyhow!("writing default config to {}: {err}", path.display())
        })?;
        tracing::info!(
            path = %path.display(),
            "wrote a default config; edit and restart to customise — starting with defaults"
        );
        AppConfig::default()
    };

    // The config module owns mutation so CLI overrides cannot bypass the same
    // validation used for TOML. The full address is applied before the port, and
    // the shipping binary fills its durable world directory last.
    config.with_runtime_overrides(bind, port, PathBuf::from(DEFAULT_WORLD_DIR))
}
