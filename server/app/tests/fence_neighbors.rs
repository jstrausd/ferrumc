//! Socket gate for fence invalidation after ordinary placement and breaking.
//!
//! Uses real creative inventory items, two observers, and the same durable redb
//! directory across a server restart. A later marker edit makes missing neighbor
//! notifications fail deterministically rather than merely exhausting a timeout.
//! Expected states come from raw Minecraft fixtures; restart assertions decode
//! network palettes without production placement, mapping, or world helpers.
//! Material/shape/gate parity and water placement are outside this gate's scope.

mod common;

use std::net::SocketAddr;
use std::time::Duration;

use anyhow::{ensure, Context};
use serde::Deserialize;
use tokio::time::timeout;

use ferrumc_app::AppConfig;
use ferrumc_codec::{write_var_int, BoundedReader};
use ferrumc_math::BlockPos;
use ferrumc_proto::generated::play::{
    ClientboundPlayPacket, PlayerAction, ServerboundSetHeldItem, SetCreativeSlot, UseItemOn,
};
use ferrumc_proto::types::BlockPosition;

use common::{encode, login_to_play, TestClient};

const GUARD: Duration = Duration::from_secs(20);
const FENCE: BlockPos = BlockPos::new(6, 64, 8);
const NEIGHBOR: BlockPos = BlockPos::new(7, 64, 8);
const MARKER: BlockPos = BlockPos::new(6, 64, 11);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BlockFixture {
    name: String,
    min_state_id: u32,
    default_state: u32,
    states: Vec<PropertyFixture>,
}

#[derive(Deserialize)]
struct PropertyFixture {
    name: String,
    #[serde(rename = "type")]
    kind: String,
    num_values: u32,
}

struct Fixtures {
    stone_item: i32,
    stone: u32,
    oak_item: i32,
    oak_isolated: u32,
    oak_east: u32,
    spruce_item: i32,
    spruce_west: u32,
    air: u32,
}

fn fixture_data() -> anyhow::Result<Fixtures> {
    let items: serde_json::Value = serde_json::from_str(include_str!(
        "../../crates/ferrumc-registry/data/items.json"
    ))?;
    let blocks: Vec<BlockFixture> = serde_json::from_str(include_str!(
        "../../crates/ferrumc-registry/data/blocks.json"
    ))?;
    let item_id = |name: &str| -> anyhow::Result<i32> {
        Ok(i32::try_from(
            items[name]["id"].as_i64().context("fixture item ID")?,
        )?)
    };
    let block = |name: &str| -> anyhow::Result<&BlockFixture> {
        blocks
            .iter()
            .find(|entry| entry.name == name)
            .with_context(|| format!("missing fixture block {name}"))
    };
    let connected = |name: &str, direction: &str| -> anyhow::Result<u32> {
        let fence = block(name)?;
        ensure!(
            fence.states.len() == 5
                && fence
                    .states
                    .iter()
                    .all(|p| p.kind == "bool" && p.num_values == 2)
                && fence.default_state == fence.min_state_id + 31,
            "unexpected {name} default/property encoding"
        );
        let index = fence
            .states
            .iter()
            .position(|p| p.name == direction)
            .context("fixture fence direction")?;
        // Boolean true precedes false in Minecraft's dense state IDs. Changing
        // exactly one default false property subtracts its encoding stride.
        let stride: u32 = fence.states[index + 1..]
            .iter()
            .map(|p| p.num_values)
            .product();
        Ok(fence.default_state - stride)
    };
    Ok(Fixtures {
        stone_item: item_id("stone")?,
        stone: block("stone")?.default_state,
        oak_item: item_id("oak_fence")?,
        oak_isolated: block("oak_fence")?.default_state,
        oak_east: connected("oak_fence", "east")?,
        spruce_item: item_id("spruce_fence")?,
        spruce_west: connected("spruce_fence", "west")?,
        air: block("air")?.default_state,
    })
}

async fn equip(client: &mut TestClient, item: i32) -> anyhow::Result<()> {
    let mut slot = Vec::new();
    for value in [1, item, 0, 0] {
        write_var_int(&mut slot, value);
    }
    client
        .send_frame(&encode(|buf| SetCreativeSlot::new(36, slot).encode(buf)))
        .await?;
    client
        .send_frame(&encode(|buf| ServerboundSetHeldItem::new(0).encode(buf)))
        .await
}

async fn place(client: &mut TestClient, target: BlockPos, sequence: i32) -> anyhow::Result<()> {
    client
        .send_frame(&encode(|buf| {
            UseItemOn::new(
                0,
                BlockPosition::new(target.x(), target.y() - 1, target.z()),
                1,
                0.5,
                1.0,
                0.5,
                false,
                false,
                sequence,
            )
            .encode(buf)
        }))
        .await
}

async fn break_block(
    client: &mut TestClient,
    target: BlockPos,
    sequence: i32,
) -> anyhow::Result<()> {
    client
        .send_frame(&encode(|buf| {
            PlayerAction::new(
                0, // Creative start-destroy is an ordinary instant block break.
                BlockPosition::new(target.x(), target.y(), target.z()),
                1,
                sequence,
            )
            .encode(buf)
        }))
        .await
}

fn record_update(
    position: BlockPos,
    state: i32,
    expected: &[(BlockPos, u32)],
    seen: &mut [bool],
) -> anyhow::Result<()> {
    for (index, &(target, wanted)) in expected.iter().enumerate() {
        if position == target {
            ensure!(
                u32::try_from(state)? == wanted,
                "BlockUpdate at {target:?}: state {state}, expected {wanted}"
            );
            seen[index] = true;
        }
    }
    Ok(())
}

async fn primary_ack(
    client: &mut TestClient,
    sequence: i32,
    expected: &[(BlockPos, u32)],
    seen: &mut [bool],
) -> anyhow::Result<()> {
    let mut acknowledged = false;
    while !acknowledged || !seen[0] {
        match client.next_play().await? {
            ClientboundPlayPacket::BlockUpdate(packet) => {
                let pos = packet.location();
                record_update(
                    BlockPos::new(pos.x(), pos.y(), pos.z()),
                    packet.block_state(),
                    expected,
                    seen,
                )?;
            }
            ClientboundPlayPacket::AcknowledgeBlockChange(packet) => {
                ensure!(packet.sequence() == sequence, "unexpected primary ACK");
                acknowledged = true;
            }
            _ => {}
        }
    }
    Ok(())
}

async fn until_marker(
    client: &mut TestClient,
    marker_state: u32,
    marker_sequence: Option<i32>,
    expected: &[(BlockPos, u32)],
    seen: &mut [bool],
) -> anyhow::Result<()> {
    let mut marker_seen = false;
    let mut acknowledged = marker_sequence.is_none();
    while !marker_seen || !acknowledged {
        match client.next_play().await? {
            ClientboundPlayPacket::BlockUpdate(packet) => {
                let pos = packet.location();
                let position = BlockPos::new(pos.x(), pos.y(), pos.z());
                if position == MARKER {
                    ensure!(
                        u32::try_from(packet.block_state())? == marker_state,
                        "marker edit was rejected or changed"
                    );
                    marker_seen = true;
                } else {
                    record_update(position, packet.block_state(), expected, seen)?;
                }
            }
            ClientboundPlayPacket::AcknowledgeBlockChange(packet) if marker_sequence.is_some() => {
                ensure!(
                    Some(packet.sequence()) == marker_sequence,
                    "unexpected marker ACK"
                );
                acknowledged = true;
            }
            _ => {}
        }
    }
    for (index, &(target, state)) in expected.iter().enumerate() {
        ensure!(
            seen[index],
            "missing BlockUpdate at {target:?} to state {state} before confirmed later marker edit"
        );
    }
    Ok(())
}

async fn spawn_chunk(client: &mut TestClient) -> anyhow::Result<Vec<u8>> {
    loop {
        if let ClientboundPlayPacket::ChunkDataAndLight(packet) = client.next_play().await? {
            if packet.x() == 0 && packet.z() == 0 {
                return Ok(packet.chunk_data().as_slice().to_vec());
            }
        }
    }
}

async fn lifecycle(addr: SocketAddr, f: &Fixtures) -> anyhow::Result<()> {
    let mut viewer = login_to_play(addr, "FenceViewer").await?;
    let mut actor = login_to_play(addr, "FenceBuilder").await?;
    common::make_creative(&mut actor).await?;
    spawn_chunk(&mut viewer).await?;
    spawn_chunk(&mut actor).await?;

    let stages = [
        (Some(f.oak_item), FENCE, vec![(FENCE, f.oak_isolated)]),
        (
            Some(f.stone_item),
            NEIGHBOR,
            vec![(NEIGHBOR, f.stone), (FENCE, f.oak_east)],
        ),
        (
            None,
            NEIGHBOR,
            vec![(NEIGHBOR, f.air), (FENCE, f.oak_isolated)],
        ),
        (
            Some(f.spruce_item),
            NEIGHBOR,
            vec![(NEIGHBOR, f.spruce_west), (FENCE, f.oak_east)],
        ),
        (
            None,
            NEIGHBOR,
            vec![(NEIGHBOR, f.air), (FENCE, f.oak_isolated)],
        ),
        (
            Some(f.stone_item),
            NEIGHBOR,
            vec![(NEIGHBOR, f.stone), (FENCE, f.oak_east)],
        ),
    ];
    let mut sequence = 0;
    for (index, (item, target, expected)) in stages.into_iter().enumerate() {
        sequence += 1;
        if let Some(item) = item {
            equip(&mut actor, item).await?;
            place(&mut actor, target, sequence).await?;
        } else {
            break_block(&mut actor, target, sequence).await?;
        }
        let mut actor_seen = vec![false; expected.len()];
        primary_ack(&mut actor, sequence, &expected, &mut actor_seen).await?;

        // Primary and secondary BlockUpdates share the Droppable/World FIFO,
        // including the actor (router::broadcast_block_update; play writer).
        // State-priority ACKs can overtake that FIFO, so both this first wait and
        // the marker wait require their actual BlockUpdates, not only the ACKs.
        sequence += 1;
        let marker_state = if index % 2 == 0 {
            equip(&mut actor, f.stone_item).await?;
            place(&mut actor, MARKER, sequence).await?;
            f.stone
        } else {
            break_block(&mut actor, MARKER, sequence).await?;
            f.air
        };
        until_marker(
            &mut actor,
            marker_state,
            Some(sequence),
            &expected,
            &mut actor_seen,
        )
        .await
        .with_context(|| format!("actor stage {index}: item {item:?} at {target:?}"))?;
        let mut viewer_seen = vec![false; expected.len()];
        until_marker(&mut viewer, marker_state, None, &expected, &mut viewer_seen)
            .await
            .with_context(|| format!("viewer stage {index}: item {item:?} at {target:?}"))?;
    }
    Ok(())
}

// Protocol 772 has no long-array length; packed entries do not cross u64 words.
fn read_palette(
    reader: &mut BoundedReader<'_>,
    entries: usize,
    indirect_max: u8,
) -> anyhow::Result<Vec<u32>> {
    let bits = reader.read_u8()?;
    if bits == 0 {
        return Ok(vec![u32::try_from(reader.read_var_int()?)?; entries]);
    }
    ensure!(bits <= 32, "invalid palette width");
    let palette = if bits <= indirect_max {
        let len = reader.read_var_int_len()?;
        ensure!(len > 0 && len <= 1usize << bits, "invalid palette length");
        let mut values = Vec::with_capacity(len);
        for _ in 0..len {
            values.push(u32::try_from(reader.read_var_int()?)?);
        }
        Some(values)
    } else {
        None
    };
    let per_word = 64 / usize::from(bits);
    let mask = (1u64 << bits) - 1;
    let mut decoded = Vec::with_capacity(entries);
    for _ in 0..entries.div_ceil(per_word) {
        let word = reader.read_u64()?;
        for index in 0..per_word.min(entries - decoded.len()) {
            let raw = u32::try_from((word >> (index * usize::from(bits))) & mask)?;
            decoded.push(match &palette {
                Some(values) => *values.get(usize::try_from(raw)?).context("palette index")?,
                None => raw,
            });
        }
    }
    Ok(decoded)
}

async fn verify_restart(addr: SocketAddr, f: &Fixtures) -> anyhow::Result<()> {
    let mut client = login_to_play(addr, "FenceReturner").await?;
    let bytes = spawn_chunk(&mut client).await?;
    let mut reader = BoundedReader::new(&bytes);
    // The final pattern is connected oak, stone, and the removed marker.
    let expected = [(FENCE, f.oak_east), (NEIGHBOR, f.stone), (MARKER, f.air)];
    for section in 0..24 {
        reader.read_u16()?;
        let states = read_palette(&mut reader, 4096, 8)?;
        read_palette(&mut reader, 64, 3)?;
        for &(pos, state) in &expected {
            if usize::try_from((pos.y() + 64) / 16)? == section {
                let index =
                    usize::try_from((pos.y().rem_euclid(16) << 8) | (pos.z() << 4) | pos.x())?;
                ensure!(
                    states[index] == state,
                    "restart at {pos:?}: {}, expected {state}",
                    states[index]
                );
            }
        }
    }
    ensure!(reader.remaining() == 0, "trailing chunk-section bytes");
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn ordinary_neighbor_placement_and_break_refresh_both_clients_and_survive_redb_restart() {
    let fixture = fixture_data().expect("independent Minecraft fixture IDs");
    let temp = tempfile::tempdir().expect("test world directory");
    let config = AppConfig::from_toml_str(
        "bind = \"127.0.0.1:0\"\nops = [\"FenceBuilder\"]\nspawn_chunk_radius = 1\nview_distance = 1\nspawn = [8.5, 64.0, 8.5]",
    )
    .expect("config parses")
    .with_world_dir(Some(temp.path().to_path_buf()))
    .expect("durable config");

    let server = ferrumc_app::run(&config).await.expect("server starts");
    let result = timeout(GUARD, lifecycle(server.local_addr(), &fixture)).await;
    // Always release owned server/storage resources, including baseline RED.
    timeout(GUARD, server.shutdown())
        .await
        .expect("first shutdown within guard")
        .expect("first clean shutdown");
    result
        .expect("lifecycle within guard")
        .expect("fence neighbor lifecycle");
    assert!(temp.path().join("world.redb").is_file(), "real redb store");

    let restarted = ferrumc_app::run(&config).await.expect("same redb reopens");
    let verified = timeout(GUARD, verify_restart(restarted.local_addr(), &fixture)).await;
    timeout(GUARD, restarted.shutdown())
        .await
        .expect("restart shutdown within guard")
        .expect("restart clean shutdown");
    verified
        .expect("restart within guard")
        .expect("persisted connected fence");
}

#[test]
fn wire_palette_decoder_rejects_malformed_data() {
    for malformed in [
        &[][..],
        &[0][..],
        &[33][..],
        &[4, 0][..],
        &[4, 17][..],
        &[4, 1, 1, 0][..],
    ] {
        assert!(read_palette(&mut BoundedReader::new(malformed), 16, 8).is_err());
    }
    let mut invalid_index = vec![4, 1, 1];
    invalid_index.extend_from_slice(&1u64.to_be_bytes());
    assert!(read_palette(&mut BoundedReader::new(&invalid_index), 16, 8).is_err());
}
