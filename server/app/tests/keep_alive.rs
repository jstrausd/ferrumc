//! Production-socket Keep Alive regressions. Reads and response fences replace
//! wall-clock sleeps; only a timeout guard bounds failures.

mod common;

use std::time::Duration;

use ferrumc_app::{AppConfig, RunningServer};
use ferrumc_codec::BoundedString;
use ferrumc_proto::generated::play::{
    ClientboundPlayPacket, ServerboundKeepAlive, TabCompleteRequest,
};
use tokio::time::timeout;

use common::{encode, login_to_play, TestClient};

const GUARD: Duration = Duration::from_secs(5);
const SENTINEL: i32 = 0x5A19;

async fn start_server() -> anyhow::Result<RunningServer> {
    let config = AppConfig::from_toml_str(
        "bind = \"127.0.0.1:0\"\nspawn_chunk_radius = 0\nview_distance = 0\n\
         keep_alive_interval_ms = 100\nio_timeout_secs = 1\n",
    )?;
    ferrumc_app::run(&config).await
}

async fn next_keep_alive(client: &mut TestClient) -> anyhow::Result<i64> {
    loop {
        if let ClientboundPlayPacket::ClientboundKeepAlive(packet) = client.next_play().await? {
            return Ok(packet.keep_alive_id());
        }
    }
}

fn sentinel(transaction: i32) -> Vec<u8> {
    encode(|buf| {
        TabCompleteRequest::new(transaction, BoundedString::new("/sp".to_owned())?).encode(buf)
    })
}

async fn expect_closed_without_sentinel(client: &mut TestClient) -> anyhow::Result<()> {
    while let Some(packet) = client.next_play_or_closed().await? {
        if let ClientboundPlayPacket::TabCompleteResponse(response) = packet {
            anyhow::ensure!(
                response.transaction_id() != SENTINEL,
                "packet following an invalid Keep Alive was processed"
            );
        }
    }
    Ok(())
}

#[tokio::test]
async fn mismatched_echo_closes_before_the_next_pipelined_packet() {
    timeout(GUARD, async {
        let server = start_server().await?;
        let mut client = login_to_play(server.local_addr(), "WrongEcho").await?;
        let id = next_keep_alive(&mut client).await?;
        let wrong = encode(|buf| ServerboundKeepAlive::new(id.wrapping_add(1)).encode(buf));
        client.send_frames(&[&wrong, &sentinel(SENTINEL)]).await?;
        expect_closed_without_sentinel(&mut client).await?;
        server.shutdown().await
    })
    .await
    .expect("mismatched echo completed within guard")
    .expect("mismatched echo failed closed");
}

#[tokio::test]
async fn matching_echo_keeps_the_connection_live_but_a_replay_closes_it() {
    timeout(GUARD, async {
        let server = start_server().await?;
        let mut client = login_to_play(server.local_addr(), "ReplayEcho").await?;
        let id = next_keep_alive(&mut client).await?;
        let echo = encode(|buf| ServerboundKeepAlive::new(id).encode(buf));
        // A response to the next packet proves the matching echo was accepted.
        client
            .send_frames(&[&echo, &sentinel(SENTINEL - 1)])
            .await?;
        loop {
            if let ClientboundPlayPacket::TabCompleteResponse(response) = client.next_play().await?
            {
                if response.transaction_id() == SENTINEL - 1 {
                    break;
                }
            }
        }
        client.send_frames(&[&echo, &sentinel(SENTINEL)]).await?;
        expect_closed_without_sentinel(&mut client).await?;
        server.shutdown().await
    })
    .await
    .expect("matching/replayed echo completed within guard")
    .expect("matching echo accepted and replay rejected");
}

#[tokio::test]
async fn timer_wakeups_cannot_renew_an_idle_connections_deadline() {
    timeout(GUARD, async {
        let server = start_server().await?;
        let mut client = login_to_play(server.local_addr(), "SilentPeer").await?;
        let mut requests = 0;
        while let Some(packet) = client.next_play_or_closed().await? {
            if matches!(packet, ClientboundPlayPacket::ClientboundKeepAlive(_)) {
                requests += 1;
                anyhow::ensure!(requests == 1, "an unanswered Keep Alive was replaced");
            }
        }
        anyhow::ensure!(
            requests == 1,
            "the peer closed before requesting a Keep Alive"
        );
        server.shutdown().await
    })
    .await
    .expect("silent peer closed within the configured I/O window despite recurring timers")
    .expect("one unanswered Keep Alive followed by clean teardown");
}
