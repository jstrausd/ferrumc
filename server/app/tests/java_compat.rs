//! Opt-in genuine Java-21/Paper-API plugin acceptance over the Rust server socket.
//!
//! Build `server/java-bridge` with Maven first, then set the three explicit
//! `FERRUMC_JAVA_*` paths and run `cargo test -p ferrumc-app --locked --test
//! java_compat -- --ignored`. An invoked test fails on missing artifacts.

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Context;
use ferrumc_app::AppConfig;
use ferrumc_codec::{BoundedReader, BoundedString};
use ferrumc_nbt::NbtTag;
use ferrumc_proto::generated::play::{ChatCommand, ClientboundPlayPacket, TabCompleteRequest};
use tokio::net::TcpListener;
use tokio::time::timeout;

use common::{encode, login_to_play, TestClient};

const GUARD: Duration = Duration::from_secs(45);

struct Artifacts {
    executable: PathBuf,
    bridge: PathBuf,
    fixture: PathBuf,
    collision: PathBuf,
}

impl Artifacts {
    fn configured() -> anyhow::Result<Self> {
        let configured_path = |key: &str| -> anyhow::Result<PathBuf> {
            let value = std::env::var_os(key).with_context(|| {
                format!("{key} is required when invoking ignored Java acceptance")
            })?;
            let path = std::fs::canonicalize(value).with_context(|| format!("invalid {key}"))?;
            anyhow::ensure!(path.is_file(), "{key} must name a real file");
            Ok(path)
        };
        let executable = configured_path("FERRUMC_JAVA_EXECUTABLE")?;
        let bridge = configured_path("FERRUMC_JAVA_BRIDGE_JAR")?;
        let fixture = configured_path("FERRUMC_JAVA_FIXTURE_JAR")?;
        let stem = fixture
            .file_stem()
            .and_then(|stem| stem.to_str())
            .context("fixture filename must be UTF-8")?;
        let collision = fixture.with_file_name(format!("{stem}-collision.jar"));
        anyhow::ensure!(
            collision.is_file(),
            "Maven's collision fixture is required: {}",
            collision.display()
        );
        Ok(Self {
            executable,
            bridge,
            fixture,
            collision,
        })
    }

    fn config(&self, plugins: &Path, bind: &str) -> anyhow::Result<AppConfig> {
        let quote = |value: &str| toml::Value::String(value.to_owned()).to_string();
        AppConfig::from_toml_str(&format!(
            "bind = {}\ncompression_threshold = -1\nspawn_chunk_radius = 0\n\
             view_distance = 1\nmax_connections = 8\nbuiltin_plugins = false\n\
             ops = ['JavaAdmin']\ndefault_permission_level = 4\n\
             keep_alive_interval_ms = 15000\nio_timeout_secs = 30\n\
             java_executable = {}\njava_bridge_jar = {}\njava_plugins_dir = {}\n\
             java_bridge_timeout_ms = 10000\n",
            quote(bind),
            quote(
                self.executable
                    .to_str()
                    .context("Java executable must be UTF-8")?
            ),
            quote(self.bridge.to_str().context("bridge JAR must be UTF-8")?),
            quote(plugins.to_str().context("plugin path must be UTF-8")?),
        ))
    }

    fn install(&self, directory: &Path, collision: bool) -> anyhow::Result<()> {
        std::fs::create_dir_all(directory)?;
        std::fs::copy(
            if collision {
                &self.collision
            } else {
                &self.fixture
            },
            directory.join("fixture.jar"),
        )?;
        Ok(())
    }
}

fn system_text(content: &NbtTag) -> anyhow::Result<&str> {
    match content {
        NbtTag::Compound(root) => match root.get("text") {
            Some(NbtTag::String(text)) => Ok(text),
            _ => anyhow::bail!("SystemChat must have literal text"),
        },
        _ => anyhow::bail!("SystemChat must have a compound component"),
    }
}

/// Decodes the actual graph's literal nodes, including the integer/string
/// argument properties emitted by `FerrumC`. Unknown parsers fail the assertion.
fn graph_literals(payload: &[u8]) -> anyhow::Result<BTreeSet<String>> {
    let mut reader = BoundedReader::new(payload);
    let count = reader.read_var_int_len()?;
    anyhow::ensure!(count <= 4096, "bounded command graph");
    let mut names = BTreeSet::new();
    for _ in 0..count {
        let flags = reader.read_u8()?;
        let children = reader.read_var_int_len()?;
        for _ in 0..children {
            reader.read_var_int()?;
        }
        if flags & 0x08 != 0 {
            reader.read_var_int()?;
        }
        match flags & 0x03 {
            0 => {}
            1 => {
                names.insert(
                    BoundedString::<32767>::read(&mut reader)?
                        .as_str()
                        .to_owned(),
                );
            }
            2 => {
                BoundedString::<32767>::read(&mut reader)?;
                match reader.read_var_int()? {
                    3 => {
                        let properties = reader.read_u8()?;
                        if properties & 1 != 0 {
                            reader.read_i32()?;
                        }
                        if properties & 2 != 0 {
                            reader.read_i32()?;
                        }
                    }
                    5 => {
                        reader.read_var_int()?;
                    }
                    parser => anyhow::bail!("unexpected graph parser {parser}"),
                }
                if flags & 0x10 != 0 {
                    BoundedString::<32767>::read(&mut reader)?;
                }
            }
            _ => anyhow::bail!("invalid command node type"),
        }
    }
    reader.read_var_int()?;
    reader.finish()?;
    Ok(names)
}

async fn complete_join(client: &mut TestClient, allowed: bool) -> anyhow::Result<()> {
    let mut graph = None;
    let mut messages = Vec::new();
    while graph.is_none() || messages.len() < 2 {
        match client.next_play().await? {
            ClientboundPlayPacket::Commands(packet) => {
                graph = Some(graph_literals(packet.payload())?);
            }
            ClientboundPlayPacket::SystemChat(chat) => {
                let text = system_text(chat.content())?;
                if text.starts_with("join:") {
                    messages.push(text.to_owned());
                }
            }
            _ => {}
        }
    }
    assert_eq!(
        messages,
        ["join:lowest", "join:monitor"],
        "real event priority order"
    );
    let graph = graph.context("Commands packet")?;
    for route in [
        "bridgefixture",
        "bfixture",
        "bridgefixture:bridgefixture",
        "bridgefixture:bfixture",
    ] {
        assert_eq!(graph.contains(route), allowed, "wire graph route {route}");
    }
    Ok(())
}

async fn command_reply(client: &mut TestClient, command: &str) -> anyhow::Result<String> {
    let command = BoundedString::<256>::new(command.to_owned())?;
    client
        .send_frame(&encode(|buffer| ChatCommand::new(command).encode(buffer)))
        .await?;
    loop {
        if let ClientboundPlayPacket::SystemChat(chat) = client.next_play().await? {
            return Ok(system_text(chat.content())?.to_owned());
        }
    }
}

async fn tab_complete(client: &mut TestClient, text: &str, id: i32) -> anyhow::Result<Vec<String>> {
    let request = TabCompleteRequest::new(id, BoundedString::new(text.to_owned())?);
    client
        .send_frame(&encode(|buffer| request.encode(buffer)))
        .await?;
    loop {
        if let ClientboundPlayPacket::TabCompleteResponse(response) = client.next_play().await? {
            assert_eq!(response.transaction_id(), id);
            return Ok(response
                .matches()
                .iter()
                .map(|entry| entry.suggestion().as_str().to_owned())
                .collect());
        }
    }
}

fn counter(directory: &Path, key: &str) -> anyhow::Result<u32> {
    let config = std::fs::read_to_string(directory.join("BridgeFixture/config.yml"))?;
    config
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}: ")))
        .with_context(|| format!("missing fixture lifecycle counter {key}"))?
        .parse()
        .map_err(Into::into)
}

async fn socket_lifecycle(artifacts: &Artifacts, directory: &Path) -> anyhow::Result<()> {
    artifacts.install(directory, false)?;
    let config = artifacts.config(directory, "127.0.0.1:0")?;
    let server = ferrumc_app::run(&config).await?;
    let address = server.local_addr();
    let flow = async {
        let mut admin = login_to_play(address, "JavaAdmin").await?;
        let canonical = admin.login_uuid().context("Login Success UUID")?;
        complete_join(&mut admin, true).await?;
        for (route, label) in [
            ("bridgefixture", "bridgefixture"),
            ("BFIXTURE", "bfixture"),
            ("bridgefixture:bfixture", "bridgefixture:bfixture"),
        ] {
            assert_eq!(
                command_reply(&mut admin, route).await?,
                format!("fixture:{label}:JavaAdmin:{canonical}:true:true:FerrumC:1.0.0")
            );
        }
        assert_eq!(tab_complete(&mut admin, "/bfi", 1).await?, ["bfixture"]);
        assert_eq!(
            command_reply(&mut admin, "bfixture unicode").await?,
            "Grüße 🌍"
        );
        for (action, method) in [("world", "getWorld"), ("scheduler", "getScheduler")] {
            let failure = command_reply(&mut admin, &format!("bfixture {action}")).await?;
            assert!(
                failure.contains(method),
                "unsupported API feedback: {failure}"
            );
            assert!(
                command_reply(&mut admin, "bfixture")
                    .await?
                    .starts_with("fixture:"),
                "unsupported callback must leave the process usable"
            );
        }
        let unicode_error = command_reply(&mut admin, "bfixture unicode-error").await?;
        assert!(
            unicode_error.contains("Java plugin failed"),
            "Unicode errors remain remote callback failures"
        );
        assert!(command_reply(&mut admin, "bfixture")
            .await?
            .starts_with("fixture:"));
        let mut guest = login_to_play(address, "JavaGuest").await?;
        complete_join(&mut guest, false).await?;
        assert_eq!(
            tab_complete(&mut guest, "/bfi", 2).await?,
            Vec::<String>::new()
        );
        assert!(
            command_reply(&mut guest, "BFIXTURE")
                .await?
                .contains("do not have permission"),
            "default permission level four cannot fabricate Java op status"
        );
        let stale = command_reply(&mut admin, "bfixture previous").await?;
        assert!(
            stale.contains("cross-player messaging is not implemented"),
            "foreign recipient refused: {stale}"
        );
        assert!(command_reply(&mut admin, "bfixture")
            .await?
            .starts_with("fixture:"));
        drop(guest);
        drop(admin);
        anyhow::Ok(canonical)
    }
    .await;
    let stopped = server.shutdown().await;
    let canonical = flow?;
    stopped?;
    for (key, expected) in [
        ("enabled", 1),
        ("disabled", 1),
        ("join-count", 2),
        ("quit-count", 2),
    ] {
        assert_eq!(counter(directory, key)?, expected, "first process {key}");
    }
    restart_plugin(&config, directory, canonical).await
}

async fn restart_plugin(
    config: &AppConfig,
    directory: &Path,
    canonical: uuid::Uuid,
) -> anyhow::Result<()> {
    let server = ferrumc_app::run(config).await?;
    let flow = async {
        let mut player = login_to_play(server.local_addr(), "JavaAdmin").await?;
        assert_eq!(
            player.login_uuid(),
            Some(canonical),
            "canonical UUID survives restart"
        );
        complete_join(&mut player, true).await?;
        assert!(command_reply(&mut player, "bridgefixture")
            .await?
            .starts_with("fixture:"));
        anyhow::Ok(())
    }
    .await;
    let stopped = server.shutdown().await;
    flow?;
    stopped?;
    for (key, expected) in [
        ("enabled", 2),
        ("disabled", 2),
        ("join-count", 3),
        ("quit-count", 3),
    ] {
        assert_eq!(
            counter(directory, key)?,
            expected,
            "restarted process {key}"
        );
    }
    Ok(())
}

async fn startup_rollback(artifacts: &Artifacts, directory: &Path) -> anyhow::Result<()> {
    artifacts.install(directory, true)?;
    let reservation = TcpListener::bind("127.0.0.1:0").await?;
    let address = reservation.local_addr()?;
    drop(reservation);
    let config = artifacts.config(directory, &address.to_string())?;
    let error = ferrumc_app::run(&config)
        .await
        .expect_err("builtin alias collision must abort startup");
    assert!(error.to_string().contains("collides"), "{error:#}");
    assert_eq!(counter(directory, "enabled")?, 1);
    assert_eq!(
        counter(directory, "disabled")?,
        1,
        "rollback disables real Java plugin once"
    );
    let listener = TcpListener::bind(address)
        .await
        .context("failed startup must release the listener")?;
    drop(listener);

    let rejected = directory.join("load-failure");
    artifacts.install(&rejected, false)?;
    std::fs::create_dir(rejected.join("BridgeFixture"))?;
    std::fs::write(
        rejected.join("BridgeFixture/config.yml"),
        "enabled: 0\ndisabled: 0\nfail-enable: true\n",
    )?;
    let config = artifacts.config(&rejected, &address.to_string())?;
    let error = ferrumc_app::run(&config)
        .await
        .expect_err("unsupported API during enable must reject startup");
    assert!(format!("{error:#}").contains("getScheduler"), "{error:#}");
    assert_eq!(counter(&rejected, "enabled")?, 1);
    assert_eq!(
        counter(&rejected, "disabled")?,
        1,
        "load failure disables once before responding"
    );
    let listener = TcpListener::bind(address)
        .await
        .context("plugin load failure must release the listener")?;
    drop(listener);
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires explicit Java executable and Maven-built bridge/ordinary fixture JAR paths"]
async fn genuine_paper_api_plugin_commands_events_permissions_restart_and_startup_rollback(
) -> anyhow::Result<()> {
    let artifacts = Artifacts::configured()?;
    let directory = tempfile::tempdir()?;
    timeout(GUARD, async {
        socket_lifecycle(&artifacts, &directory.path().join("plugins")).await?;
        startup_rollback(&artifacts, &directory.path().join("collision")).await
    })
    .await
    .context("genuine Java plugin socket acceptance timed out")?
}
