//! Real sockets prove that well-framed ignored traffic cannot hold a login slot.
//! Exact timing and cancellation boundaries use the paused unit-test clock.

mod common;

use std::net::SocketAddr;
use std::time::Duration;

use ferrumc_app::{AppConfig, RunningServer};
use ferrumc_codec::{write_var_int, BoundedReader, BoundedString};
use ferrumc_proto::generated::configuration::{
    ClientboundConfigurationPacket, ServerboundKnownPacks,
};
use ferrumc_proto::generated::handshake::Handshake;
use ferrumc_proto::generated::login::{ClientboundLoginPacket, LoginAcknowledged, LoginStart};
use ferrumc_proto::generated::status::StatusRequest;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

use common::{encode, login_to_play};

const GUARD: Duration = Duration::from_secs(15);

#[derive(Clone, Copy, Debug)]
enum WaitingFor {
    Handshake,
    LoginStart,
    LoginAck,
    KnownPacks,
    FinishAck,
    StatusPing,
}

async fn server() -> anyhow::Result<RunningServer> {
    let config = AppConfig::from_toml_str(
        "bind = \"127.0.0.1:0\"\ncompression_threshold = -1\n\
         spawn_chunk_radius = 0\nview_distance = 0\nio_timeout_secs = 1\n\
         max_connections = 1\nbuiltin_plugins = false\n\
         [access]\nper_ip_connection_limit = 1\n",
    )?;
    ferrumc_app::run(&config).await
}

async fn send(stream: &mut TcpStream, body: &[u8]) -> anyhow::Result<()> {
    let mut bytes = Vec::new();
    write_var_int(&mut bytes, i32::try_from(body.len())?);
    bytes.extend_from_slice(body);
    stream.write_all(&bytes).await?;
    Ok(())
}

async fn next_frame(stream: &mut TcpStream) -> anyhow::Result<Vec<u8>> {
    let mut size = 0_u32;
    for shift in (0..35).step_by(7) {
        let byte = stream.read_u8().await?;
        size |= u32::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            anyhow::ensure!(size > 0 && size <= 1_048_576, "bounded server frame");
            let mut body = vec![0; usize::try_from(size)?];
            stream.read_exact(&mut body).await?;
            return Ok(body);
        }
    }
    anyhow::bail!("invalid frame length")
}

async fn stalled_socket(address: SocketAddr, phase: WaitingFor) -> anyhow::Result<TcpStream> {
    let mut stream = TcpStream::connect(address).await?;
    if matches!(phase, WaitingFor::Handshake) {
        return Ok(stream);
    }
    let host = BoundedString::new("127.0.0.1".to_owned())?;
    send(
        &mut stream,
        &encode(|out| {
            Handshake::new(
                772,
                host,
                address.port(),
                if matches!(phase, WaitingFor::StatusPing) {
                    1
                } else {
                    2
                },
            )
            .encode(out)
        }),
    )
    .await?;
    if matches!(phase, WaitingFor::StatusPing) {
        send(&mut stream, &encode(|out| StatusRequest.encode(out))).await?;
        next_frame(&mut stream).await?;
        return Ok(stream);
    }
    if matches!(phase, WaitingFor::LoginStart) {
        return Ok(stream);
    }
    send(
        &mut stream,
        &encode(|out| {
            LoginStart::new(
                BoundedString::new("DeadlineProbe".to_owned()).expect("bounded fixture"),
                uuid::Uuid::nil(),
            )
            .encode(out)
        }),
    )
    .await?;
    let login = next_frame(&mut stream).await?;
    let mut reader = BoundedReader::new(&login);
    let id = reader.read_var_int()?;
    assert!(matches!(
        ClientboundLoginPacket::decode(id, &mut reader)?,
        ClientboundLoginPacket::LoginSuccess(_)
    ));
    reader.finish()?;
    if matches!(phase, WaitingFor::LoginAck) {
        return Ok(stream);
    }
    send(&mut stream, &encode(|out| LoginAcknowledged.encode(out))).await?;
    let known = next_frame(&mut stream).await?;
    let mut reader = BoundedReader::new(&known);
    let id = reader.read_var_int()?;
    assert!(matches!(
        ClientboundConfigurationPacket::decode(id, &mut reader)?,
        ClientboundConfigurationPacket::ClientboundKnownPacks(_)
    ));
    reader.finish()?;
    if matches!(phase, WaitingFor::KnownPacks) {
        return Ok(stream);
    }
    send(
        &mut stream,
        &encode(|out| ServerboundKnownPacks::new(Vec::new()).encode(out)),
    )
    .await?;
    loop {
        let frame = next_frame(&mut stream).await?;
        let mut reader = BoundedReader::new(&frame);
        let id = reader.read_var_int()?;
        if matches!(
            ClientboundConfigurationPacket::decode(id, &mut reader)?,
            ClientboundConfigurationPacket::FinishConfiguration(_)
        ) {
            reader.finish()?;
            return Ok(stream);
        }
        reader.finish()?;
    }
}

#[tokio::test]
async fn every_pre_play_phase_releases_a_stalled_connection_slot() {
    timeout(GUARD, async {
        for phase in [
            WaitingFor::Handshake,
            WaitingFor::LoginStart,
            WaitingFor::LoginAck,
            WaitingFor::KnownPacks,
            WaitingFor::FinishAck,
            WaitingFor::StatusPing,
        ] {
            let server = server().await?;
            let flow = async {
                let mut stream = stalled_socket(server.local_addr(), phase).await?;
                let mut byte = [0];
                assert_eq!(stream.read(&mut byte).await?, 0, "stalled phase {phase:?}");
                drop(stream);
                let client = login_to_play(server.local_addr(), "FreshSlot").await?;
                drop(client);
                anyhow::Ok(())
            }
            .await;
            let stopped = server.shutdown().await;
            flow?;
            stopped?;
        }
        anyhow::Ok(())
    })
    .await
    .expect("stalled phases finish under guard")
    .expect("absolute deadlines release capacity");
}

#[tokio::test]
async fn continuous_ignored_frames_do_not_extend_login_or_configuration_admission() {
    timeout(GUARD, async {
        for phase in [WaitingFor::LoginAck, WaitingFor::KnownPacks, WaitingFor::FinishAck] {
            let server = server().await?;
            let flow = async {
                let stream = stalled_socket(server.local_addr(), phase).await?;
                let (mut reader, mut writer) = stream.into_split();
                // 64 complete unknown-id frames per write. On the old reset-per-
                // read behavior this traffic held admission forever. No timer or
                // wall-clock sleep in the sender masks a server timeout.
                let burst = [1_u8, 0x7f].repeat(64);
                let mut byte = [0];
                loop {
                    tokio::select! {
                        biased;
                        read = reader.read(&mut byte) => {
                            match read {
                                Ok(0) => {},
                                Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset => {},
                                Ok(_) => anyhow::bail!("ignored traffic received a response"),
                                Err(error) => return Err(error.into()),
                            }
                            break;
                        }
                        write = writer.write_all(&burst) => {
                            if write.is_err() { break; }
                            tokio::task::yield_now().await;
                        }
                    }
                }
                drop(reader);
                drop(writer);
                let client = login_to_play(server.local_addr(), "FreshAfterFlood").await?;
                drop(client);
                anyhow::Ok(())
            }.await;
            let stopped = server.shutdown().await;
            flow?;
            stopped?;
        }
        anyhow::Ok(())
    }).await.expect("flood must not hold login beyond guard").expect("ignored frames do not refresh progress");
}
