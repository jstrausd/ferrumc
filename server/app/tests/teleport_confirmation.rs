//! Teleport confirmations authorize the newest server-issued position only.

mod common;

use std::time::Duration;

use ferrumc_app::{AppConfig, RunningServer};
use ferrumc_codec::BoundedString;
use ferrumc_core::PlayerId;
use ferrumc_math::Vec3;
use ferrumc_nbt::NbtTag;
use ferrumc_proto::generated::play::{
    ChatCommand, ClientboundPlayPacket, ConfirmTeleportation, SetPlayerPositionAndRotation,
    SynchronizePlayerPosition, TabCompleteRequest,
};
use tokio::time::timeout;

use common::{encode, login_to_play_unconfirmed, TestClient};

const GUARD: Duration = Duration::from_secs(15);
const SPAWN: Vec3 = Vec3::new(8.0, 64.0, 8.0);
const MOVE: Vec3 = Vec3::new(12.0, 64.0, 9.0);

async fn start_server() -> anyhow::Result<RunningServer> {
    ferrumc_app::run(&AppConfig::from_toml_str(
        "bind = \"127.0.0.1:0\"\nspawn_chunk_radius = 0\nview_distance = 0\n\
         ops = [\"TeleportGuard\"]\n",
    )?)
    .await
}

async fn next_sync(client: &mut TestClient) -> anyhow::Result<SynchronizePlayerPosition> {
    loop {
        if let ClientboundPlayPacket::SynchronizePlayerPosition(sync) = client.next_play().await? {
            return Ok(sync);
        }
    }
}

fn confirm(id: i32) -> Vec<u8> {
    encode(|buf| ConfirmTeleportation::new(id).encode(buf))
}

fn movement(position: Vec3, yaw: f32, pitch: f32) -> Vec<u8> {
    encode(|buf| {
        SetPlayerPositionAndRotation::new(position.x, position.y, position.z, yaw, pitch, 0)
            .encode(buf)
    })
}

fn command(text: &str) -> Vec<u8> {
    encode(|buf| ChatCommand::new(BoundedString::new(text.to_owned())?).encode(buf))
}

fn sentinel(id: i32) -> Vec<u8> {
    encode(|buf| TabCompleteRequest::new(id, BoundedString::new("/sp".to_owned())?).encode(buf))
}

#[tokio::test]
async fn released_frame_responses_flush_without_a_followup_read_or_timer() {
    timeout(GUARD, async {
        // A periodic UpdateTime after 20 ticks must not accidentally flush the
        // response. One TPS keeps it beyond the one-second progress deadline,
        // just like the explicitly slow chunk and Keep Alive timers.
        let config = AppConfig::from_toml_str(
            "bind = \"127.0.0.1:0\"\nspawn_chunk_radius = 0\nview_distance = 0\n\
             ops = [\"TeleportGuard\"]\nticks_per_second = 1\nio_timeout_secs = 1\n\
             chunk_stream_interval_ms = 60000\nkeep_alive_interval_ms = 15000\n",
        )?;
        for request in [sentinel(781), command("spawn")] {
            let server = ferrumc_app::run(&config).await?;
            let mut client =
                login_to_play_unconfirmed(server.local_addr(), "TeleportGuard").await?;
            let initial = next_sync(&mut client).await?;
            // Driver acceptance queues /tp's sync before the next frame. The
            // held request is released by the last output turn. Send nothing
            // afterwards: another socket read must not repair the missing flush.
            client
                .send_frames(&[
                    &confirm(initial.teleport_id()),
                    &command("tp 20 64 8"),
                    &request,
                ])
                .await?;
            let teleport = next_sync(&mut client).await?;
            anyhow::ensure!(
                teleport.x().to_bits() == 20.0_f64.to_bits(),
                "first routed teleport missing"
            );
            loop {
                match client.next_play().await? {
                    ClientboundPlayPacket::TabCompleteResponse(response)
                        if response.transaction_id() == 781 =>
                    {
                        break
                    }
                    ClientboundPlayPacket::SynchronizePlayerPosition(sync)
                        if sync.teleport_id() != teleport.teleport_id()
                            && sync.x().to_bits() == SPAWN.x.to_bits() =>
                    {
                        break
                    }
                    _ => {}
                }
            }
            drop(client);
            server.shutdown().await?;
        }
        Ok::<(), anyhow::Error>(())
    })
    .await
    .expect("released response flow within guard")
    .expect("responses generated after the prefix flush reach the socket");
}

/// A driver-owned time reply fences earlier inbound processing before a fresh
/// simulation snapshot checks the authoritative coordinate.
async fn assert_position(
    server: &RunningServer,
    client: &mut TestClient,
    expected: Vec3,
) -> anyhow::Result<()> {
    client.send_frame(&command("time query daytime")).await?;
    loop {
        if let ClientboundPlayPacket::SystemChat(chat) = client.next_play().await? {
            if let NbtTag::Compound(root) = chat.content() {
                if matches!(root.get("text"), Some(NbtTag::String(text)) if text.starts_with("The time is "))
                {
                    break;
                }
            }
        }
    }
    let snapshots = server.snapshot_handle();
    let after = snapshots.latest().tick;
    loop {
        let snapshot = snapshots.latest();
        if snapshot.tick > after {
            let player = snapshot
                .players
                .iter()
                .find(|row| row.player_id == PlayerId::offline("TeleportGuard").as_uuid().as_u128())
                .ok_or_else(|| anyhow::anyhow!("player left before position assertion"))?;
            anyhow::ensure!(
                player.position.x.to_bits() == expected.x.to_bits()
                    && player.position.y.to_bits() == expected.y.to_bits()
                    && player.position.z.to_bits() == expected.z.to_bits(),
                "authoritative position ({}, {}, {}) differs from expected {expected:?}",
                player.position.x,
                player.position.y,
                player.position.z,
            );
            return Ok(());
        }
        tokio::task::yield_now().await;
    }
}

#[tokio::test]
async fn initial_position_waits_for_matching_confirmation() {
    timeout(GUARD, async {
        let server = start_server().await?;
        let mut client = login_to_play_unconfirmed(server.local_addr(), "TeleportGuard").await?;
        let sync = next_sync(&mut client).await?;
        client.send_frame(&movement(MOVE, 31.0, -17.0)).await?;
        assert_position(&server, &mut client, SPAWN).await?;
        client
            .send_frames(&[
                &confirm(sync.teleport_id() + 99),
                &movement(MOVE, 31.0, -17.0),
            ])
            .await?;
        assert_position(&server, &mut client, SPAWN).await?;
        client
            .send_frames(&[&confirm(sync.teleport_id()), &movement(MOVE, 31.0, -17.0)])
            .await?;
        assert_position(&server, &mut client, MOVE).await?;
        drop(client);
        server.shutdown().await
    })
    .await
    .expect("teleport flow completed within guard")
    .expect("pending initial teleport gates position");
}

#[tokio::test]
async fn command_teleport_supersedes_initial_and_stale_confirmations() {
    timeout(GUARD, async {
        let server = start_server().await?;
        let mut client = login_to_play_unconfirmed(server.local_addr(), "TeleportGuard").await?;
        let initial = next_sync(&mut client).await?;
        client.send_frame(&command("spawn")).await?;
        let spawn = next_sync(&mut client).await?;
        anyhow::ensure!(
            spawn.teleport_id() != initial.teleport_id(),
            "spawn reused join ID"
        );
        client
            .send_frames(&[
                &confirm(initial.teleport_id()),
                &movement(MOVE, 40.0, -10.0),
            ])
            .await?;
        assert_position(&server, &mut client, SPAWN).await?;
        client
            .send_frames(&[&confirm(spawn.teleport_id()), &movement(MOVE, 40.0, -10.0)])
            .await?;
        assert_position(&server, &mut client, MOVE).await?;
        drop(client);
        server.shutdown().await
    })
    .await
    .expect("supersession within guard")
    .expect("newest inline teleport wins");
}

#[tokio::test]
async fn routed_tp_fences_a_move_in_the_same_tcp_write() {
    timeout(GUARD, async {
        let server = start_server().await?;
        let mut client = login_to_play_unconfirmed(server.local_addr(), "TeleportGuard").await?;
        let initial = next_sync(&mut client).await?;
        client.send_frame(&confirm(initial.teleport_id())).await?;
        client
            .send_frames(&[&command("tp 20 64 8"), &movement(MOVE, 55.0, -20.0)])
            .await?;
        let first = next_sync(&mut client).await?;
        anyhow::ensure!(
            first.teleport_id() != initial.teleport_id(),
            "routed tp reused join ID"
        );
        assert_position(&server, &mut client, Vec3::new(20.0, 64.0, 8.0)).await?;
        // An earlier ID must stay stale even when another /tp and ACK share a read.
        client
            .send_frames(&[
                &command("tp 24 64 8"),
                &confirm(first.teleport_id()),
                &movement(MOVE, 66.0, -21.0),
            ])
            .await?;
        let second = next_sync(&mut client).await?;
        anyhow::ensure!(
            second.teleport_id() != first.teleport_id(),
            "superseding tp reused ID"
        );
        assert_position(&server, &mut client, Vec3::new(24.0, 64.0, 8.0)).await?;
        client
            .send_frames(&[&confirm(second.teleport_id()), &movement(MOVE, 66.0, -21.0)])
            .await?;
        assert_position(&server, &mut client, MOVE).await?;
        drop(client);
        server.shutdown().await
    })
    .await
    .expect("pipelined tp within guard")
    .expect("routed teleport fences subsequent movement");
}

#[tokio::test]
async fn replay_of_current_id_closes_before_pipelined_sentinel() {
    timeout(GUARD, async {
        let server = start_server().await?;
        let mut client = login_to_play_unconfirmed(server.local_addr(), "TeleportGuard").await?;
        let initial = next_sync(&mut client).await?;
        client.send_frames(&[&confirm(initial.teleport_id()), &confirm(initial.teleport_id()), &sentinel(77)]).await?;
        while let Some(packet) = client.next_play_or_closed().await? {
            anyhow::ensure!(!matches!(packet, ClientboundPlayPacket::TabCompleteResponse(response) if response.transaction_id() == 77), "replay failed to stop next frame");
        }
        server.shutdown().await
    }).await.expect("replay closure within guard").expect("current-ID replay terminates");
}

#[tokio::test]
async fn pending_teleport_position_and_allowed_look_survive_restart_without_ack() {
    timeout(GUARD, async {
        let directory = tempfile::tempdir()?;
        let config = AppConfig::from_toml_str(
            "bind = \"127.0.0.1:0\"\nspawn_chunk_radius = 0\nview_distance = 0\nops = [\"TeleportGuard\"]\n",
        )?.with_world_dir(Some(directory.path().to_owned()))?;
        let server = ferrumc_app::run(&config).await?;
        let mut client = login_to_play_unconfirmed(server.local_addr(), "TeleportGuard").await?;
        let initial = next_sync(&mut client).await?;
        client.send_frame(&confirm(initial.teleport_id())).await?;
        client.send_frames(&[&command("tp 20 64 8"), &movement(MOVE, 73.0, -31.0)]).await?;
        let teleport = next_sync(&mut client).await?;
        anyhow::ensure!(teleport.x().to_bits() == 20.0_f64.to_bits(), "tp packet wrong");
        assert_position(&server, &mut client, Vec3::new(20.0, 64.0, 8.0)).await?;
        // Deliberately never ACK. The position is already server-authoritative;
        // the pending movement can rotate but cannot replace that coordinate.
        drop(client);
        server.shutdown().await?;
        let server = ferrumc_app::run(&config).await?;
        let mut client = login_to_play_unconfirmed(server.local_addr(), "TeleportGuard").await?;
        let restored = next_sync(&mut client).await?;
        anyhow::ensure!((restored.x().to_bits(), restored.y().to_bits(), restored.z().to_bits()) == (20.0_f64.to_bits(), 64.0_f64.to_bits(), 8.0_f64.to_bits()), "pending teleport position was lost across restart");
        anyhow::ensure!((restored.yaw().to_bits(), restored.pitch().to_bits()) == (73.0_f32.to_bits(), (-31.0_f32).to_bits()), "Paper's allowed pending look was lost");
        drop(client);
        server.shutdown().await
    }).await.expect("no-ACK restart within guard").expect("server teleport persists before client acknowledgement");
}
