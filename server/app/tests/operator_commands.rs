//! Actual player authorization, live command graphs, local recovery and restart.
//! Expectations parse the received graph without the production command builder.
//! Unsigned command packets only; signed commands and client op `EntityEvent` parity
//! remain outside this app-only gate.

mod common;

use std::collections::BTreeSet;
use std::net::SocketAddr;
use std::time::Duration;

use anyhow::{ensure, Context};
use ferrumc_app::{AppConfig, RunningServer};
use ferrumc_codec::{BoundedReader, BoundedString};
use ferrumc_nbt::NbtTag;
use ferrumc_proto::generated::play::{ChatCommand, ClientboundPlayPacket};
use tokio::time::timeout;

use common::{encode, login_to_play, TestClient};

const GUARD: Duration = Duration::from_secs(20);

fn text(tag: &NbtTag) -> anyhow::Result<&str> {
    let NbtTag::Compound(fields) = tag else {
        anyhow::bail!("chat compound");
    };
    let Some(NbtTag::String(text)) = fields.get("text") else {
        anyhow::bail!("chat text");
    };
    Ok(text)
}

fn string(reader: &mut BoundedReader<'_>) -> anyhow::Result<String> {
    let len = reader.read_var_int_len()?;
    ensure!(len <= 32767, "graph string bound");
    Ok(std::str::from_utf8(reader.read_bytes(len)?)?.to_owned())
}

fn root_literals(bytes: &[u8]) -> anyhow::Result<BTreeSet<String>> {
    let mut reader = BoundedReader::new(bytes);
    let count = reader.read_var_int_len()?;
    ensure!(count > 0 && count <= 1024, "graph count");
    let mut nodes = Vec::with_capacity(count);
    for _ in 0..count {
        let flags = reader.read_u8()?;
        let len = reader.read_var_int_len()?;
        ensure!(len <= count, "child count");
        let mut children = Vec::with_capacity(len);
        for _ in 0..len {
            let index = reader.read_var_int_len()?;
            ensure!(index < count, "child index");
            children.push(index);
        }
        if flags & 8 != 0 {
            ensure!(reader.read_var_int_len()? < count, "redirect index");
        }
        let name = match flags & 3 {
            0 => None,
            1 => Some(string(&mut reader)?),
            2 => {
                let name = string(&mut reader)?;
                match reader.read_var_int()? {
                    3 => {
                        let limits = reader.read_u8()?;
                        ensure!(limits & !3 == 0, "integer bounds");
                        if limits & 1 != 0 {
                            reader.read_i32()?;
                        }
                        if limits & 2 != 0 {
                            reader.read_i32()?;
                        }
                    }
                    5 => ensure!((0..=2).contains(&reader.read_var_int()?), "string mode"),
                    _ => anyhow::bail!("unexpected app parser"),
                }
                Some(name)
            }
            _ => anyhow::bail!("invalid node type"),
        };
        if flags & 16 != 0 {
            string(&mut reader)?;
        }
        nodes.push((flags & 3, name, children));
    }
    let root = reader.read_var_int_len()?;
    let (kind, _, children) = nodes.get(root).context("root index")?;
    ensure!(
        *kind == 0 && reader.remaining() == 0,
        "root/trailing graph data"
    );
    children
        .iter()
        .map(|index| {
            let (kind, name, _) = &nodes[*index];
            ensure!(*kind == 1, "root child must be a literal");
            name.clone().context("literal name")
        })
        .collect()
}

async fn graph(client: &mut TestClient, operator: bool) -> anyhow::Result<()> {
    loop {
        if let ClientboundPlayPacket::Commands(packet) = client.next_play().await? {
            let roots = root_literals(packet.payload())?;
            ensure!(roots.contains("spawn"), "ordinary command retained");
            let visible = roots.contains("op");
            for command in ["gamemode", "op", "deop"] {
                ensure!(
                    roots.contains(command) == visible,
                    "{command} visibility must match operator commands"
                );
            }
            // Watches coalesce commits, and command feedback reads may consume
            // intermediate graphs. Wait for the state transition, not a count.
            if visible == operator {
                return Ok(());
            }
        }
    }
}

async fn gamemode(client: &mut TestClient, mode: &str, id: f32) -> anyhow::Result<()> {
    let packet = ChatCommand::new(BoundedString::<256>::new(format!("gamemode {mode}"))?);
    client.send_frame(&encode(|buf| packet.encode(buf))).await?;
    let mut feedback_seen = false;
    let mut event_seen = false;
    while !feedback_seen || !event_seen {
        match client.next_play().await? {
            ClientboundPlayPacket::SystemChat(chat) => {
                ensure!(
                    text(chat.content())?.contains("Game mode set to"),
                    "gamemode success feedback"
                );
                feedback_seen = true;
            }
            ClientboundPlayPacket::GameEvent(event) if event.reason() == 3 => {
                ensure!(
                    event.value().to_bits() == id.to_bits(),
                    "actual game mode event"
                );
                event_seen = true;
            }
            _ => {}
        }
    }
    Ok(())
}

async fn restored_mode(client: &mut TestClient) -> anyhow::Result<()> {
    loop {
        if let ClientboundPlayPacket::GameEvent(event) = client.next_play().await? {
            if event.reason() == 3 {
                ensure!(
                    event.value().to_bits() == 1.0_f32.to_bits(),
                    "restored creative game mode"
                );
                return Ok(());
            }
        }
    }
}

async fn command(client: &mut TestClient, command: &str, wanted: &str) -> anyhow::Result<()> {
    let packet = ChatCommand::new(BoundedString::<256>::new(command.to_owned())?);
    client.send_frame(&encode(|buf| packet.encode(buf))).await?;
    loop {
        match client.next_play().await? {
            ClientboundPlayPacket::SystemChat(chat) => {
                let feedback = text(chat.content())?;
                ensure!(
                    feedback.contains(wanted),
                    "{command}: unexpected feedback {feedback:?}"
                );
                return Ok(());
            }
            ClientboundPlayPacket::GameEvent(event)
                if wanted == "permission denied" && event.reason() == 3 =>
            {
                anyhow::bail!("denied command changed game mode");
            }
            _ => {}
        }
    }
}

async fn first_lifecycle(addr: SocketAddr) -> anyhow::Result<()> {
    let mut admin = login_to_play(addr, "Admin").await?;
    let mut member = login_to_play(addr, "Member").await?;
    graph(&mut admin, true).await?;
    graph(&mut member, false).await?;
    command(&mut member, "op Member", "permission denied").await?;
    command(&mut admin, "op Member", "Member is now an operator").await?;
    graph(&mut member, true).await?;
    gamemode(&mut member, "survival", 0.0).await?;
    gamemode(&mut member, "creative", 1.0).await?;
    command(&mut admin, "deop Member", "Member is no longer an operator").await?;
    graph(&mut member, false).await?;
    command(&mut member, "gamemode survival", "permission denied").await?;
    command(&mut member, "op Member", "permission denied").await?;
    command(&mut admin, "op Keeper", "Keeper is now an operator").await?;
    command(&mut admin, "deop Admin", "Admin is no longer an operator").await?;
    graph(&mut admin, false).await?;
    command(&mut admin, "op Admin", "permission denied").await?;
    Ok(())
}

async fn after_restart(server: &RunningServer) -> anyhow::Result<()> {
    let mut admin = login_to_play(server.local_addr(), "Admin").await?;
    let mut member = login_to_play(server.local_addr(), "Member").await?;
    let mut keeper = login_to_play(server.local_addr(), "Keeper").await?;
    graph(&mut admin, false).await?;
    graph(&mut member, false).await?;
    graph(&mut keeper, true).await?;
    restored_mode(&mut admin).await?;
    restored_mode(&mut member).await?;
    command(&mut admin, "op Admin", "permission denied").await?;
    ensure!(
        server
            .execute_admin_command("op Member")
            .await?
            .contains("now an operator"),
        "local recovery"
    );
    graph(&mut member, true).await?;
    ensure!(
        server
            .execute_admin_command("deop Keeper")
            .await?
            .contains("no longer an operator"),
        "local revoke"
    );
    graph(&mut keeper, false).await?;
    command(&mut keeper, "op Keeper", "permission denied").await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn live_operator_commands_revoke_permissions_and_persist_across_restart() {
    let temp = tempfile::tempdir().expect("world directory");
    let config = AppConfig::from_toml_str(
        "bind='127.0.0.1:0'\nspawn_chunk_radius=1\nbuiltin_plugins=false\nops=['Admin']",
    )
    .expect("config")
    .with_world_dir(Some(temp.path().to_path_buf()))
    .expect("durable config");
    let server = ferrumc_app::run(&config).await.expect("server");
    let result = timeout(GUARD, first_lifecycle(server.local_addr())).await;
    timeout(GUARD, server.shutdown())
        .await
        .expect("shutdown guard")
        .expect("clean shutdown");
    result
        .expect("lifecycle guard")
        .expect("operator wire lifecycle");
    let restarted = ferrumc_app::run(&config)
        .await
        .expect("same seed config restarts");
    let result = timeout(GUARD, after_restart(&restarted)).await;
    timeout(GUARD, restarted.shutdown())
        .await
        .expect("restart shutdown guard")
        .expect("clean restart shutdown");
    result
        .expect("restart guard")
        .expect("persisted identities and local recovery");
}

#[test]
fn graph_parser_rejects_malformed_and_out_of_range_data() {
    for bytes in [
        &[][..],
        &[0][..],
        &[1, 3, 0][..],
        &[1, 0, 1, 1, 0][..],
        &[1, 0, 0, 1][..],
    ] {
        assert!(root_literals(bytes).is_err());
    }
    assert!(root_literals(&[1, 0, 0, 0])
        .expect("single root")
        .is_empty());
}
