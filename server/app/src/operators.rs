//! App-owned operator identities and durable overrides of configuration seeds.
//!
//! Offline names are case-sensitive, matching the server's offline UUID rule.
//! This is a bounded `FerrumC` record, not the Vanilla `ops.json` format. Java
//! mutation awaits a separate permission-refresh RPC and is deliberately refused.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

use anyhow::{ensure, Context};
use ferrumc_core::PlayerId;
use ferrumc_net::LivenessDeadline;
use serde::{Deserialize, Serialize};
use tokio::sync::{watch, Mutex};

use crate::AppConfig;

/// Identity gate is additional to level 3: a high default level cannot grant ops.
pub(crate) const ADMIN_PERMISSION: &str = "ferrumc.admin.operators";
const MAX_OPERATORS: usize = 1024;
const MAX_RECORD_BYTES: u64 = 128 * 1024;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Override {
    name: String,
    operator: bool,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema: u8,
    overrides: Vec<Override>,
}

/// The only live operator source; writes are serialized and published after save.
#[derive(Debug)]
pub(crate) struct Operators {
    seeds: BTreeSet<PlayerId>,
    overrides: RwLock<BTreeMap<PlayerId, Override>>,
    mutation: Arc<Mutex<()>>,
    revisions: watch::Sender<u64>,
    path: Option<PathBuf>,
    java_enabled: bool,
    closed: AtomicBool,
    failed: AtomicBool,
    fatal_shutdown: RwLock<Option<watch::Sender<bool>>>,
}

impl Operators {
    /// Reads durable overrides without rewriting them or resurrecting config ops.
    pub(crate) fn from_config(config: &AppConfig) -> anyhow::Result<Arc<Self>> {
        ensure!(
            config.ops().len() <= MAX_OPERATORS,
            "too many configured operators"
        );
        let mut seeds = BTreeSet::new();
        for name in config.ops() {
            validate_name(name)?;
            seeds.insert(PlayerId::offline(name));
        }
        let path = config.world_dir().map(|dir| dir.join("operators.json"));
        let overrides = path.as_deref().map(load).transpose()?.unwrap_or_default();
        let (revisions, _) = watch::channel(0);
        Ok(Arc::new(Self {
            seeds,
            overrides: RwLock::new(overrides),
            mutation: Arc::new(Mutex::new(())),
            revisions,
            path,
            java_enabled: config.java_plugins_dir().is_some(),
            closed: AtomicBool::new(false),
            failed: AtomicBool::new(false),
            fatal_shutdown: RwLock::new(None),
        }))
    }

    /// Returns the current identity status, failing closed on poisoned state.
    pub(crate) fn contains(&self, player: PlayerId) -> bool {
        if self.failed.load(Ordering::Acquire) {
            return false;
        }
        self.overrides.read().is_ok_and(|overrides| {
            overrides
                .get(&player)
                .map_or_else(|| self.seeds.contains(&player), |entry| entry.operator)
        })
    }

    /// Coalesced notifications for live client command-graph refresh.
    pub(crate) fn subscribe(&self) -> watch::Receiver<u64> {
        self.revisions.subscribe()
    }

    /// Activation needs persistence and a complete live permission consumer path.
    pub(crate) fn mutations_supported(&self) -> bool {
        self.path.is_some() && !self.java_enabled
    }

    /// Connects uncertain durability to the server's existing fatal-stop path.
    pub(crate) fn attach_shutdown(&self, shutdown: watch::Sender<bool>) -> anyhow::Result<()> {
        *self
            .fatal_shutdown
            .write()
            .map_err(|_| anyhow::anyhow!("operator shutdown state poisoned"))? = Some(shutdown);
        Ok(())
    }

    /// Commits a local-admin (`None`) or currently authorized player's change.
    ///
    /// Admission can be cancelled before the lock. Once accepted, an owned task
    /// keeps serialization through commit/publication even if its caller leaves.
    pub(crate) async fn change(
        self: &Arc<Self>,
        issuer: Option<PlayerId>,
        name: &str,
        operator: bool,
    ) -> anyhow::Result<String> {
        validate_name(name)?;
        let guard = Arc::clone(&self.mutation).lock_owned().await;
        ensure!(
            !self.closed.load(Ordering::Acquire),
            "server is shutting down"
        );
        let state = Arc::clone(self);
        let name = name.to_owned();
        tokio::spawn(async move {
            let _guard = guard;
            state.commit(issuer, &name, operator).await
        })
        .await
        .context("operator mutation task failed")?
    }

    async fn commit(
        &self,
        issuer: Option<PlayerId>,
        name: &str,
        operator: bool,
    ) -> anyhow::Result<String> {
        ensure!(
            issuer.is_none_or(|player| self.contains(player)),
            "operator identity required"
        );
        ensure!(
            !self.java_enabled,
            "operator changes with Java plugins require a permission-refresh RPC"
        );
        let path = self
            .path
            .clone()
            .context("operator changes require a persistent world_dir")?;
        let player = PlayerId::offline(name);
        ensure!(
            self.contains(player) != operator,
            "{name} is already {}",
            if operator {
                "an operator"
            } else {
                "not an operator"
            }
        );
        let mut candidate = self
            .overrides
            .read()
            .map_err(|_| anyhow::anyhow!("operator state poisoned"))?
            .clone();
        ensure!(
            candidate.contains_key(&player) || candidate.len() < MAX_OPERATORS,
            "operator override limit reached"
        );
        candidate.insert(
            player,
            Override {
                name: name.to_owned(),
                operator,
            },
        );
        let record = Record {
            schema: 1,
            overrides: candidate.values().cloned().collect(),
        };
        let saved = tokio::task::spawn_blocking(move || save(&path, &record))
            .await
            .context("operator save task failed")?;
        self.publish(candidate, saved)?;
        Ok(format!(
            "{name} is {}",
            if operator {
                "now an operator"
            } else {
                "no longer an operator"
            }
        ))
    }

    /// Storage completion and publication share one failure boundary.
    fn publish(
        &self,
        candidate: BTreeMap<PlayerId, Override>,
        saved: Result<(), SaveFailure>,
    ) -> anyhow::Result<()> {
        if let Err(failure) = saved {
            if failure.replaced {
                // Rename completed but directory durability failed: the on-disk
                // version is uncertain. Deny operator identity and stop admission.
                self.failed.store(true, Ordering::Release);
                self.closed.store(true, Ordering::Release);
                self.revisions
                    .send_modify(|revision| *revision = revision.wrapping_add(1));
                if let Ok(shutdown) = self.fatal_shutdown.read() {
                    if let Some(shutdown) = &*shutdown {
                        let _ = shutdown.send(true);
                    }
                }
            }
            return Err(failure.error);
        }
        *self
            .overrides
            .write()
            .map_err(|_| anyhow::anyhow!("operator state poisoned after save"))? = candidate;
        self.revisions
            .send_modify(|revision| *revision = revision.wrapping_add(1));
        Ok(())
    }

    /// Closes admission before any await; accepted owned commits still finish.
    pub(crate) fn begin_close(&self) {
        self.closed.store(true, Ordering::Release);
    }

    /// Drains the accepted commit while queued changes observe closed admission.
    pub(crate) async fn close(&self) {
        self.begin_close();
        let _guard = self.mutation.lock().await;
    }

    /// Reports uncertain durability even after all regular tasks were drained.
    pub(crate) fn verify_shutdown(&self) -> anyhow::Result<()> {
        ensure!(
            !self.failed.load(Ordering::Acquire),
            "operator persistence became uncertain"
        );
        Ok(())
    }
}

fn validate_name(name: &str) -> anyhow::Result<()> {
    ensure!(
        !name.is_empty()
            && name.len() <= 16
            && name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'),
        "target must be one exact 1-16 character offline player name"
    );
    Ok(())
}

/// Strict single-target syntax, shared by the local console and player handler.
pub(crate) fn parse_command(command: &str) -> anyhow::Result<(&str, bool)> {
    let mut tokens = command.trim_start_matches('/').split_whitespace();
    let operator = match tokens.next() {
        Some("op") => true,
        Some("deop") => false,
        _ => anyhow::bail!("supported admin commands: op <name>, deop <name>"),
    };
    let name = tokens.next().context("usage: op <name> | deop <name>")?;
    ensure!(
        tokens.next().is_none(),
        "exactly one target name is supported"
    );
    validate_name(name)?;
    Ok((name, operator))
}

/// Cancel waiting callers at stop/deadline; accepted owned commits still drain.
/// The inner result is a recoverable command failure, the outer one ends Play.
pub(crate) async fn guard_change(
    deadline: Option<LivenessDeadline>,
    mut shutdown: watch::Receiver<bool>,
    work: impl std::future::Future<Output = anyhow::Result<String>>,
) -> anyhow::Result<anyhow::Result<String>> {
    ensure!(
        !*shutdown.borrow(),
        "server shutdown during operator command"
    );
    let deadline = deadline.context("Play liveness is closed")?;
    tokio::select! {
        biased;
        _ = shutdown.changed() => Err(anyhow::anyhow!("server shutdown during operator command")),
        () = tokio::time::sleep_until(deadline.at()) => Err(deadline.timeout().into()),
        result = work => Ok(result),
    }
}

fn load(path: &Path) -> anyhow::Result<BTreeMap<PlayerId, Override>> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(error) => return Err(error).context("opening operator overrides"),
    };
    let mut bytes = Vec::new();
    file.take(MAX_RECORD_BYTES + 1).read_to_end(&mut bytes)?;
    ensure!(
        u64::try_from(bytes.len())? <= MAX_RECORD_BYTES,
        "operator record is too large"
    );
    let record: Record = serde_json::from_slice(&bytes).context("invalid operator record")?;
    ensure!(
        record.schema == 1 && record.overrides.len() <= MAX_OPERATORS,
        "unsupported operator schema or override count"
    );
    let mut overrides = BTreeMap::new();
    for entry in record.overrides {
        validate_name(&entry.name)?;
        ensure!(
            overrides
                .insert(PlayerId::offline(&entry.name), entry)
                .is_none(),
            "duplicate operator identity"
        );
    }
    Ok(overrides)
}

struct SaveFailure {
    error: anyhow::Error,
    replaced: bool,
}

fn save(path: &Path, record: &Record) -> Result<(), SaveFailure> {
    let mut replaced = false;
    let result = save_inner(path, record, &mut replaced, File::sync_all);
    result.map_err(|error| SaveFailure { error, replaced })
}

fn save_inner(
    path: &Path,
    record: &Record,
    replaced: &mut bool,
    sync_directory: impl FnOnce(&File) -> std::io::Result<()>,
) -> anyhow::Result<()> {
    let parent = path
        .parent()
        .context("operator record has no parent directory")?;
    let bytes = serde_json::to_vec(record)?;
    ensure!(
        u64::try_from(bytes.len())? <= MAX_RECORD_BYTES,
        "operator record is too large"
    );
    // An exclusive unique staging file avoids following leftover/symlink files.
    let staging = parent.join(format!(".operators-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| -> anyhow::Result<()> {
        let directory = File::open(parent)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staging)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        fs::rename(&staging, path)?;
        *replaced = true;
        sync_directory(&directory)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(staging);
    }
    result.context("committing operator overrides")
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::task::{Context as TaskContext, Poll, Waker};

    use super::*;

    fn config(dir: &Path, extra: &str) -> AppConfig {
        AppConfig::from_toml_str(&format!("ops = [\"Admin\", \"Owner\"]\n{extra}"))
            .expect("valid config")
            .with_world_dir(Some(dir.to_path_buf()))
            .expect("persistent config")
    }

    #[tokio::test]
    async fn config_seeds_are_overridden_by_durable_grants_and_revocations() {
        let dir = tempfile::tempdir().expect("directory");
        let cfg = config(dir.path(), "");
        let operators = Operators::from_config(&cfg).expect("load seeds");
        let admin = PlayerId::offline("Admin");
        assert!(operators.contains(admin));
        operators
            .change(Some(admin), "Member", true)
            .await
            .expect("grant saved");
        operators
            .change(Some(admin), "Admin", false)
            .await
            .expect("self revoke saved");
        assert!(!operators.contains(admin));
        assert!(operators.change(Some(admin), "Other", true).await.is_err());
        let restarted = Operators::from_config(&cfg).expect("same seed config reloads");
        assert!(
            !restarted.contains(admin),
            "config cannot resurrect a revoked op"
        );
        assert!(restarted.contains(PlayerId::offline("Member")));
        assert!(
            !restarted.contains(PlayerId::offline("member")),
            "case-sensitive offline identity"
        );
        assert!(
            restarted.change(None, "Admin", true).await.is_ok(),
            "local recovery is available"
        );
    }

    #[tokio::test]
    async fn queued_admin_change_rechecks_identity_after_earlier_revocation() {
        let dir = tempfile::tempdir().expect("directory");
        let operators = Operators::from_config(&config(dir.path(), "")).expect("seeds");
        let held = Arc::clone(&operators.mutation).lock_owned().await;
        let mut revoke =
            Box::pin(operators.change(Some(PlayerId::offline("Owner")), "Admin", false));
        let mut promote =
            Box::pin(operators.change(Some(PlayerId::offline("Admin")), "Member", true));
        let mut context = TaskContext::from_waker(Waker::noop());
        assert!(matches!(revoke.as_mut().poll(&mut context), Poll::Pending));
        assert!(matches!(promote.as_mut().poll(&mut context), Poll::Pending));
        drop(held);
        let (revoked, promoted) = tokio::join!(revoke, promote);
        assert!(revoked.is_ok());
        assert!(promoted
            .expect_err("queued former admin loses authority")
            .to_string()
            .contains("operator identity"));
        assert!(!operators.contains(PlayerId::offline("Member")));
    }

    #[tokio::test]
    async fn accepted_change_finishes_when_caller_is_cancelled_and_shutdown_drains_it() {
        let dir = tempfile::tempdir().expect("directory");
        let cfg = config(dir.path(), "");
        let operators = Operators::from_config(&cfg).expect("seeds");
        let mut change = Box::pin(operators.change(None, "Member", true));
        assert!(matches!(
            change
                .as_mut()
                .poll(&mut TaskContext::from_waker(Waker::noop())),
            Poll::Pending
        ));
        drop(change);
        operators.close().await;
        operators.verify_shutdown().expect("durable accepted write");
        assert!(operators.contains(PlayerId::offline("Member")));
        assert!(Operators::from_config(&cfg)
            .expect("reopen")
            .contains(PlayerId::offline("Member")));
        assert!(operators.change(None, "Other", true).await.is_err());
    }

    #[tokio::test]
    async fn stop_preserves_accepted_commit_but_refuses_already_queued_mutation() {
        let dir = tempfile::tempdir().expect("directory");
        let cfg = config(dir.path(), "");
        let operators = Operators::from_config(&cfg).expect("seeds");
        let mut accepted = Box::pin(operators.change(None, "Member", true));
        let mut queued = Box::pin(operators.change(None, "Other", true));
        let mut context = TaskContext::from_waker(Waker::noop());
        // This current-thread fixture cannot poll the owned commit yet. The
        // first future acquired the mutex; the second is only queued for it.
        assert!(matches!(
            accepted.as_mut().poll(&mut context),
            Poll::Pending
        ));
        assert!(matches!(queued.as_mut().poll(&mut context), Poll::Pending));
        operators.begin_close();
        assert!(operators.closed.load(Ordering::Acquire));
        let (accepted, queued, ()) = tokio::join!(accepted, queued, operators.close());
        accepted.expect("accepted mutation persists across stop");
        assert!(queued
            .expect_err("queued mutation refused")
            .to_string()
            .contains("shutting down"));
        let reopened = Operators::from_config(&cfg).expect("durable reopen");
        assert!(reopened.contains(PlayerId::offline("Member")));
        assert!(!reopened.contains(PlayerId::offline("Other")));
    }

    #[tokio::test(start_paused = true)]
    async fn queued_player_change_cancels_at_keep_alive_deadline_without_mutation() {
        use ferrumc_net::{ConnectionLiveness, ConnectionState, LivenessConfig, LivenessTimeout};
        use std::time::Duration;
        use tokio::time::{advance, Instant};

        let dir = tempfile::tempdir().expect("directory");
        let cfg = config(dir.path(), "");
        let operators = Operators::from_config(&cfg).expect("seeds");
        let held = Arc::clone(&operators.mutation).lock_owned().await;
        let mut liveness = ConnectionLiveness::new(
            Instant::now(),
            LivenessConfig::new(
                Duration::from_secs(30),
                Duration::from_secs(30),
                Duration::from_secs(15),
            ),
        );
        liveness
            .enter_state(ConnectionState::Play, Instant::now())
            .expect("Play");
        liveness.keep_alive_sent(77, Instant::now()).expect("sent");
        let (_signal, shutdown) = watch::channel(false);
        let state = Arc::clone(&operators);
        let task = tokio::spawn(async move {
            guard_change(
                liveness.next_deadline(),
                shutdown,
                state.change(None, "Member", true),
            )
            .await
        });
        tokio::task::yield_now().await;
        advance(Duration::from_secs(14)).await;
        assert!(!task.is_finished());
        advance(Duration::from_secs(1)).await;
        let error = task.await.expect("guard task").expect_err("deadline");
        assert_eq!(
            error.downcast_ref(),
            Some(&LivenessTimeout::KeepAlive { id: 77 })
        );
        drop(held);
        operators.close().await;
        assert!(!operators.contains(PlayerId::offline("Member")));
        assert!(!dir.path().join("operators.json").exists());
    }

    #[tokio::test]
    async fn guarded_player_change_observes_shutdown_and_preserves_command_errors() {
        use ferrumc_net::{ConnectionLiveness, ConnectionState, LivenessConfig};
        use std::time::Duration;
        use tokio::time::Instant;

        let mut liveness = ConnectionLiveness::new(
            Instant::now(),
            LivenessConfig::new(
                Duration::from_secs(30),
                Duration::from_secs(30),
                Duration::from_secs(15),
            ),
        );
        liveness
            .enter_state(ConnectionState::Play, Instant::now())
            .expect("Play");
        let (signal, shutdown) = watch::channel(false);
        let error = guard_change(
            liveness.next_deadline(),
            shutdown.clone(),
            std::future::ready(Err(anyhow::anyhow!("save refused"))),
        )
        .await
        .expect("recoverable command result")
        .expect_err("save error");
        assert_eq!(error.to_string(), "save refused");
        let task = tokio::spawn(guard_change(
            liveness.next_deadline(),
            shutdown,
            std::future::pending(),
        ));
        tokio::task::yield_now().await;
        signal.send(true).expect("stop");
        assert!(task
            .await
            .expect("task")
            .expect_err("cancelled")
            .to_string()
            .contains("shutdown"));
    }

    #[tokio::test]
    async fn failed_replacement_preserves_live_state_and_last_durable_record() {
        let dir = tempfile::tempdir().expect("directory");
        let cfg = config(dir.path(), "");
        let operators = Operators::from_config(&cfg).expect("seeds");
        operators
            .change(None, "Member", true)
            .await
            .expect("first durable state");
        let path = dir.path().join("operators.json");
        let backup = dir.path().join("backup.json");
        fs::rename(&path, &backup).expect("retain old committed record");
        fs::create_dir(&path).expect("inject replacement failure");
        let mut revisions = operators.subscribe();
        assert!(operators.change(None, "Other", true).await.is_err());
        assert!(operators.contains(PlayerId::offline("Member")));
        assert!(!operators.contains(PlayerId::offline("Other")));
        assert!(!revisions.has_changed().expect("revision channel live"));
        assert_eq!(
            fs::read_dir(dir.path()).expect("directory").count(),
            2,
            "staging file was cleaned"
        );
        fs::remove_dir(&path).expect("remove fault");
        fs::rename(&backup, &path).expect("restore durable record");
        let reopened = Operators::from_config(&cfg).expect("reopen");
        assert!(reopened.contains(PlayerId::offline("Member")));
        assert!(!reopened.contains(PlayerId::offline("Other")));
        revisions.borrow_and_update();
    }

    #[tokio::test]
    async fn failed_directory_sync_after_real_rename_denies_ops_and_requests_fatal_stop() {
        let dir = tempfile::tempdir().expect("directory");
        let cfg = config(dir.path(), "");
        let operators = Operators::from_config(&cfg).expect("seeds");
        let (signal, shutdown) = watch::channel(false);
        operators.attach_shutdown(signal).expect("fatal signal");
        let revisions = operators.subscribe();
        let entry = Override {
            name: "Member".to_owned(),
            operator: true,
        };
        let candidate = BTreeMap::from([(PlayerId::offline("Member"), entry.clone())]);
        let record = Record {
            schema: 1,
            overrides: vec![entry],
        };
        let path = dir.path().join("operators.json");
        let mut replaced = false;
        let error = save_inner(&path, &record, &mut replaced, |_| {
            Err(std::io::Error::other("injected directory sync failure"))
        })
        .expect_err("post-rename failure");
        assert!(
            replaced,
            "real rename already replaced the durable namespace"
        );
        assert!(load(&path)
            .expect("visible replacement")
            .contains_key(&PlayerId::offline("Member")));
        assert!(operators
            .publish(candidate, Err(SaveFailure { error, replaced }))
            .is_err());
        for name in ["Admin", "Owner", "Member"] {
            assert!(
                !operators.contains(PlayerId::offline(name)),
                "{name} fails closed"
            );
        }
        assert!(operators.failed.load(Ordering::Acquire));
        assert!(operators.closed.load(Ordering::Acquire));
        assert!(*shutdown.borrow(), "coordinated fatal stop requested");
        assert!(revisions.has_changed().expect("revision sender live"));
        assert!(operators.verify_shutdown().is_err());
        assert!(operators.change(None, "Other", true).await.is_err());
        assert_eq!(
            fs::read_dir(dir.path()).expect("directory").count(),
            1,
            "no staging residue"
        );
    }

    #[tokio::test]
    async fn elevated_default_level_never_grants_operator_mutation_authority() {
        let dir = tempfile::tempdir().expect("directory");
        let cfg = config(dir.path(), "default_permission_level = 4");
        let (policy, _) = crate::plugins::build_play_policy(&cfg).expect("policy");
        let member = PlayerId::offline("Member");
        assert_eq!(policy.permission_level(member), 4);
        assert!(!policy.is_operator(member));
        assert!(policy
            .operators()
            .change(Some(member), "Member", true)
            .await
            .is_err());
        let source = ferrumc_command::CommandSource::for_player(member, "Member", 4);
        assert!(policy
            .command_tree()
            .dispatch_with("op Member", &source, &|_| false)
            .is_err());
    }

    #[test]
    fn malformed_records_and_invalid_targets_fail_closed_without_rewriting() {
        let dir = tempfile::tempdir().expect("directory");
        let path = dir.path().join("operators.json");
        for record in [
            r#"{"schema":2,"overrides":[]}"#,
            r#"{"schema":1,"overrides":[],"extra":true}"#,
            r#"{"schema":1,"overrides":[{"name":"Admin","operator":"true"}]}"#,
            r#"{"schema":1,"overrides":[{"name":"bad name","operator":true}]}"#,
            r#"{"schema":1,"overrides":[{"name":"Admin","operator":true},{"name":"Admin","operator":false}]}"#,
        ] {
            fs::write(&path, record).expect("fixture");
            assert!(Operators::from_config(&config(dir.path(), "")).is_err());
            assert_eq!(fs::read_to_string(&path).expect("unchanged record"), record);
        }
        fs::write(
            &path,
            vec![b' '; usize::try_from(MAX_RECORD_BYTES).expect("bound") + 1],
        )
        .expect("oversized record");
        assert!(load(&path).is_err());
        for command in [
            "op",
            "deop",
            "op @a",
            "op Bad-Name",
            "op Admin Other",
            "op ThisNameIsTooLong1",
            "other Admin",
        ] {
            assert!(parse_command(command).is_err(), "{command}");
        }
        assert_eq!(parse_command("op Admin").expect("valid"), ("Admin", true));
    }

    #[tokio::test]
    async fn missing_persistence_refuses_activation_and_mutation() {
        let operators = Operators::from_config(&AppConfig::default()).expect("memory seed state");
        assert!(!operators.mutations_supported());
        assert!(operators
            .change(None, "Member", true)
            .await
            .expect_err("no pretend durability")
            .to_string()
            .contains("world_dir"));
    }

    #[tokio::test]
    async fn java_configuration_refuses_mutation_until_live_refresh_is_integrated() {
        let dir = tempfile::tempdir().expect("directory");
        let cfg = config(
            dir.path(),
            "java_plugins_dir='/unused/plugins'\njava_bridge_jar='/unused/bridge.jar'",
        );
        let operators = Operators::from_config(&cfg).expect("seeds without launching Java");
        assert!(!operators.mutations_supported());
        assert!(operators
            .change(None, "Member", true)
            .await
            .expect_err("guard")
            .to_string()
            .contains("permission-refresh"));
        assert!(!operators.contains(PlayerId::offline("Member")));
        assert!(!dir.path().join("operators.json").exists());
    }
}
