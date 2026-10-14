//! Real inventory-item placement, observer replication, and redb restart gate.
//!
//! Every torch is placed against a stone support first placed over the socket.
//! Expected IDs come directly from the vendored Minecraft 1.21.8 item/block
//! snapshots, never the production item-to-block mapping or placement helpers.
//! Restart verification decodes received chunk palettes instead of regenerating
//! an expected chunk with the server's world encoder. This positive-case gate
//! does not claim support rejection, fallback placement, physics, or Paper parity.

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
    ClientboundPlayPacket, ServerboundSetHeldItem, SetCreativeSlot, UseItemOn,
};
use ferrumc_proto::types::BlockPosition;

use common::{encode, login_to_play, TestClient};

const GUARD: Duration = Duration::from_secs(20);
const MIN_Y: i32 = -64;
const SECTION_COUNT: usize = 24;
const BLOCKS_PER_SECTION: usize = 4096;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BlockFixture {
    name: String,
    min_state_id: u32,
    max_state_id: u32,
    default_state: u32,
    states: Vec<PropertyFixture>,
}

#[derive(Deserialize)]
struct PropertyFixture {
    name: String,
    num_values: u32,
    #[serde(default)]
    values: Vec<String>,
}

struct TorchFixture {
    name: &'static str,
    item: i32,
    floor: u32,
    // Canonical packet face order: north, south, west, east (2, 3, 4, 5).
    wall: [u32; 4],
    support: BlockPos,
}

fn fixture_data() -> anyhow::Result<(i32, u32, Vec<TorchFixture>)> {
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
    let mut torches = Vec::new();
    for (name, wall_name, support) in [
        ("torch", "wall_torch", BlockPos::new(6, 64, 6)),
        ("soul_torch", "soul_wall_torch", BlockPos::new(10, 64, 6)),
        (
            "redstone_torch",
            "redstone_wall_torch",
            BlockPos::new(8, 64, 10),
        ),
    ] {
        let floor = block(name)?;
        let wall = block(wall_name)?;
        let facing = wall.states.first().context("wall-facing fixture")?;
        ensure!(
            facing.name == "facing" && facing.values == ["north", "south", "west", "east"],
            "unexpected {wall_name} facing order"
        );
        // For redstone, lit=true is the first boolean value and each facing has
        // two states; plain/soul wall torches have only the facing property.
        let stride = match wall.states.as_slice() {
            [_] => 1,
            [_, lit] if lit.name == "lit" && lit.num_values == 2 => 2,
            _ => anyhow::bail!("unexpected {wall_name} properties"),
        };
        ensure!(
            floor.default_state == floor.min_state_id
                && wall.max_state_id - wall.min_state_id + 1 == 4 * stride,
            "unexpected {name} fixture state range"
        );
        torches.push(TorchFixture {
            name,
            item: item_id(name)?,
            floor: floor.default_state,
            wall: std::array::from_fn(|index| {
                wall.min_state_id + u32::try_from(index).expect("four facings") * stride
            }),
            support,
        });
    }
    Ok((item_id("stone")?, block("stone")?.default_state, torches))
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

async fn click_face(
    client: &mut TestClient,
    support: BlockPos,
    face: i32,
    sequence: i32,
) -> anyhow::Result<()> {
    let (cursor_x, cursor_y, cursor_z) = match face {
        1 => (0.5, 1.0, 0.5),
        2 => (0.5, 0.5, 0.0),
        3 => (0.5, 0.5, 1.0),
        4 => (0.0, 0.5, 0.5),
        5 => (1.0, 0.5, 0.5),
        _ => anyhow::bail!("unexpected test face {face}"),
    };
    client
        .send_frame(&encode(|buf| {
            UseItemOn::new(
                0,
                BlockPosition::new(support.x(), support.y(), support.z()),
                face,
                cursor_x,
                cursor_y,
                cursor_z,
                false,
                false,
                sequence,
            )
            .encode(buf)
        }))
        .await
}

async fn observe_edit(
    client: &mut TestClient,
    target: BlockPos,
    expected: u32,
    sequence: Option<i32>,
) -> anyhow::Result<()> {
    let mut updated = false;
    let mut acknowledged = sequence.is_none();
    while !updated || !acknowledged {
        match client.next_play().await? {
            ClientboundPlayPacket::BlockUpdate(packet)
                if packet.location().x() == target.x()
                    && packet.location().y() == target.y()
                    && packet.location().z() == target.z() =>
            {
                ensure!(
                    u32::try_from(packet.block_state())? == expected,
                    "BlockUpdate at {target:?}: state {}, expected {expected}",
                    packet.block_state()
                );
                updated = true;
            }
            ClientboundPlayPacket::AcknowledgeBlockChange(packet) if sequence.is_some() => {
                ensure!(Some(packet.sequence()) == sequence, "unexpected edit ACK");
                acknowledged = true;
            }
            _ => {}
        }
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

async fn place_matrix(
    addr: SocketAddr,
    stone_item: i32,
    stone_state: u32,
    torches: &[TorchFixture],
) -> anyhow::Result<Vec<(BlockPos, u32)>> {
    let mut viewer = login_to_play(addr, "TorchViewer").await?;
    let mut actor = login_to_play(addr, "TorchBuilder").await?;
    common::make_creative(&mut actor).await?;
    spawn_chunk(&mut viewer).await?;
    spawn_chunk(&mut actor).await?;
    let mut sequence = 0;
    let mut expected = Vec::new();

    for torch in torches {
        equip(&mut actor, stone_item).await?;
        sequence += 1;
        let ground = BlockPos::new(torch.support.x(), 63, torch.support.z());
        click_face(&mut actor, ground, 1, sequence).await?;
        observe_edit(&mut actor, torch.support, stone_state, Some(sequence)).await?;
        observe_edit(&mut viewer, torch.support, stone_state, None).await?;
        expected.push((torch.support, stone_state));

        equip(&mut actor, torch.item).await?;
        for (face, offset, state) in [
            (1, (0, 1, 0), torch.floor),
            (2, (0, 0, -1), torch.wall[0]),
            (3, (0, 0, 1), torch.wall[1]),
            (4, (-1, 0, 0), torch.wall[2]),
            (5, (1, 0, 0), torch.wall[3]),
        ] {
            sequence += 1;
            let target = BlockPos::new(
                torch.support.x() + offset.0,
                torch.support.y() + offset.1,
                torch.support.z() + offset.2,
            );
            click_face(&mut actor, torch.support, face, sequence).await?;
            observe_edit(&mut actor, target, state, Some(sequence))
                .await
                .with_context(|| format!("{} item {}, face {face}", torch.name, torch.item))?;
            observe_edit(&mut viewer, target, state, None).await?;
            expected.push((target, state));
        }
    }
    Ok(expected)
}

// Protocol 772 omits the packed-long count. Entries never span u64 words.
// This deliberately reads the received wire representation without world APIs.
fn read_palette(
    reader: &mut BoundedReader<'_>,
    entries: usize,
    indirect_max: u8,
) -> anyhow::Result<Vec<u32>> {
    let bits = reader.read_u8()?;
    if bits == 0 {
        return Ok(vec![u32::try_from(reader.read_var_int()?)?; entries]);
    }
    ensure!(bits <= 32, "invalid palette width {bits}");
    let palette = if bits <= indirect_max {
        let len = reader.read_var_int_len()?;
        ensure!(len > 0 && len <= 1usize << bits, "invalid palette length");
        let mut palette = Vec::with_capacity(len);
        for _ in 0..len {
            palette.push(u32::try_from(reader.read_var_int()?)?);
        }
        Some(palette)
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
                Some(palette) => *palette
                    .get(usize::try_from(raw)?)
                    .context("palette index")?,
                None => raw,
            });
        }
    }
    Ok(decoded)
}

async fn verify_restart(addr: SocketAddr, expected: &[(BlockPos, u32)]) -> anyhow::Result<()> {
    let mut client = login_to_play(addr, "TorchReturner").await?;
    let bytes = spawn_chunk(&mut client).await?;
    let mut reader = BoundedReader::new(&bytes);
    for section in 0..SECTION_COUNT {
        reader.read_u16()?; // Non-air count precedes both palettes.
        let states = read_palette(&mut reader, BLOCKS_PER_SECTION, 8)?;
        read_palette(&mut reader, 64, 3)?; // Biomes must also be consumed.
        for &(pos, state) in expected {
            if usize::try_from((pos.y() - MIN_Y) / 16)? == section {
                let index =
                    usize::try_from((pos.y().rem_euclid(16) << 8) | (pos.z() << 4) | pos.x())?;
                ensure!(
                    states[index] == state,
                    "restart chunk at {pos:?}: state {}, expected {state}",
                    states[index]
                );
            }
        }
    }
    ensure!(reader.remaining() == 0, "trailing section bytes");
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn actual_torch_items_place_on_supported_faces_and_survive_redb_restart() {
    let (stone_item, stone_state, torches) = fixture_data().expect("Minecraft fixture IDs");
    let temp = tempfile::tempdir().expect("test world directory");
    let config = AppConfig::from_toml_str("bind = \"127.0.0.1:0\"\nops = [\"TorchBuilder\"]\nspawn_chunk_radius = 1\nview_distance = 1")
        .expect("config parses")
        .with_world_dir(Some(temp.path().to_path_buf()))
        .expect("durable config");

    let server = ferrumc_app::run(&config).await.expect("server starts");
    let placed = timeout(
        GUARD,
        place_matrix(server.local_addr(), stone_item, stone_state, &torches),
    )
    .await;
    // Even the baseline RED must release its listener/store before reporting.
    timeout(GUARD, server.shutdown())
        .await
        .expect("first shutdown within guard")
        .expect("first clean shutdown");
    let expected = placed
        .expect("placement matrix within guard")
        .expect("actual torch-item placement matrix");
    assert!(temp.path().join("world.redb").is_file(), "real redb store");

    let restarted = ferrumc_app::run(&config).await.expect("same redb reopens");
    let verified = timeout(GUARD, verify_restart(restarted.local_addr(), &expected)).await;
    timeout(GUARD, restarted.shutdown())
        .await
        .expect("restart shutdown within guard")
        .expect("restart clean shutdown");
    verified
        .expect("restart verification within guard")
        .expect("all observer-confirmed states survive restart");
}

#[test]
fn wire_palette_decoder_rejects_malformed_data() {
    for malformed in [&[][..], &[0][..], &[33][..], &[4, 0][..], &[4, 17][..]] {
        assert!(read_palette(&mut BoundedReader::new(malformed), 16, 8).is_err());
    }
    // One palette entry, but the first packed nibble names index 1.
    let mut invalid_index = vec![4, 1, 1];
    invalid_index.extend_from_slice(&1u64.to_be_bytes());
    assert!(read_palette(&mut BoundedReader::new(&invalid_index), 16, 8).is_err());
    // A valid header with a truncated packed word must also fail closed.
    assert!(read_palette(&mut BoundedReader::new(&[4, 1, 1, 0]), 16, 8).is_err());
}
