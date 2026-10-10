//! Real-socket component snapshots survive two complete server restarts.
mod common;
#[path = "common/player_payload.rs"]
mod player_payload;

use std::path::Path;
use std::time::Duration;

use ferrumc_app::AppConfig;
use ferrumc_codec::write_var_int;
use ferrumc_core::PlayerId;
use ferrumc_items::{ComponentValue, ItemId};
use ferrumc_nbt::{write_network_root, NbtCompound, NbtLimits, NbtTag};
use ferrumc_proto::generated::play::{ClientboundPlayPacket, SetCreativeSlot, WindowClick};
use ferrumc_storage::{PlayerStore, RedbStore};
use tokio::runtime::Builder;
use tokio::time::timeout;

use common::{encode, login_to_play, TestClient};

const GUARD: Duration = Duration::from_secs(20);
const NAME: &str = "ComponentPlayer";

fn persistent_config(world: &Path) -> AppConfig {
    AppConfig::from_toml_str("bind = \"127.0.0.1:0\"\nspawn_chunk_radius = 0\nview_distance = 0\nbuiltin_plugins = false")
        .expect("config")
        .with_world_dir(Some(world.to_path_buf()))
        .expect("world path")
}

async fn content(client: &mut TestClient) -> anyhow::Result<Vec<u8>> {
    loop {
        if let ClientboundPlayPacket::SetContainerContent(packet) = client.next_play().await? {
            anyhow::ensure!(packet.window_id() == 0, "expected player screen");
            return Ok(packet.payload().to_vec());
        }
    }
}

fn text(label: &str) -> NbtTag {
    let mut compound = NbtCompound::new();
    compound.push("text", NbtTag::String(label.to_string()));
    NbtTag::Compound(compound)
}

fn fixture(item_name: &str, count: u8, label: Option<&str>) -> anyhow::Result<Vec<u8>> {
    let item =
        ItemId::from_name(item_name).ok_or_else(|| anyhow::anyhow!("unknown fixture item"))?;
    let mut out = Vec::new();
    write_var_int(&mut out, i32::from(count));
    write_var_int(&mut out, item.id());
    write_var_int(&mut out, if label.is_some() { 2 } else { 0 });
    write_var_int(&mut out, i32::from(label.is_some()));
    if let Some(label) = label {
        let name = write_network_root(&text(label), &NbtLimits::default())?;
        for (id, payload) in [(5, name), (3, vec![3])] {
            write_var_int(&mut out, id);
            write_var_int(&mut out, i32::try_from(payload.len())?);
            out.extend_from_slice(&payload);
        }
        write_var_int(&mut out, 4); // preserve removal of unbreakable
    }
    Ok(out)
}

async fn creative(client: &mut TestClient, slot: i16, item: Vec<u8>) -> anyhow::Result<i32> {
    client
        .send_frame(&encode(|out| SetCreativeSlot::new(slot, item).encode(out)))
        .await?;
    loop {
        if let ClientboundPlayPacket::SetContainerSlot(packet) = client.next_play().await? {
            anyhow::ensure!(packet.slot() == slot, "wrong creative echo");
            return Ok(packet.state_id());
        }
    }
}

async fn seed_and_stop(config: &AppConfig) -> anyhow::Result<Vec<u8>> {
    let server = ferrumc_app::run(config).await?;
    let mut client = login_to_play(server.local_addr(), NAME).await?;
    let _ = content(&mut client).await?;
    for slot in 9..45 {
        creative(&mut client, slot, fixture("stone", 64, None)?).await?;
    }
    creative(
        &mut client,
        5,
        fixture("diamond_helmet", 1, Some("S02 helmet"))?,
    )
    .await?;
    creative(&mut client, 45, fixture("emerald", 4, Some("S02 offhand"))?).await?;
    let state = creative(&mut client, 1, fixture("emerald", 7, Some("S02 cursor"))?).await?;
    let mut rest = 1_i16.to_be_bytes().to_vec();
    rest.push(0);
    for value in [0, 0, 0] {
        write_var_int(&mut rest, value);
    }
    client
        .send_frame(&encode(|out| WindowClick::new(0, state, rest).encode(out)))
        .await?;
    let expected = content(&mut client).await?;
    // The live cursor remains held at stop because all thirty-six bag slots are full.
    server.shutdown().await?;
    drop(client);
    Ok(expected)
}

async fn restore_and_stop(config: &AppConfig, expected: &[u8]) -> anyhow::Result<()> {
    let server = ferrumc_app::run(config).await?;
    let mut client = login_to_play(server.local_addr(), NAME).await?;
    let actual = content(&mut client).await?;
    anyhow::ensure!(
        actual == expected,
        "all forty-six slots and cursor must retain their exact trusted wire representation"
    );
    server.shutdown().await?;
    drop(client);
    Ok(())
}

fn assert_component_stack(stack: &ferrumc_items::ItemStack, count: u8, label: &str) {
    assert_eq!(stack.count(), count);
    assert_eq!(
        stack.components().added(),
        &[
            ComponentValue::CustomName(text(label)),
            ComponentValue::Damage(3)
        ]
    );
    assert_eq!(
        stack
            .components()
            .removed()
            .iter()
            .map(|id| id.get())
            .collect::<Vec<_>>(),
        vec![4]
    );
}

#[test]
fn complete_slot_and_carried_components_survive_two_server_restarts() {
    let temp = tempfile::tempdir().expect("isolated world");
    let config = persistent_config(temp.path());
    let runtime = Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let expected = runtime
        .block_on(async { timeout(GUARD, seed_and_stop(&config)).await })
        .expect("bounded seed")
        .expect("seed and save");
    for _ in 0..2 {
        let store = RedbStore::open_with_generation_metadata(temp.path().join("world.redb"))
            .expect("reopen durable player store");
        let record = runtime
            .block_on(store.load_player(PlayerId::offline(NAME)))
            .expect("load")
            .expect("saved record");
        let saved = player_payload::decode(&record).expect("schema three");
        assert_eq!(saved.slots.len(), 46);
        assert_component_stack(&saved.slots[5], 1, "S02 helmet");
        assert_component_stack(&saved.slots[45], 4, "S02 offhand");
        assert_component_stack(&saved.carried, 7, "S02 cursor");
        assert!(saved.slots[9..45]
            .iter()
            .all(|stack| stack.count() == 64 && stack.item() == ItemId::from_name("stone")));
        drop(store);
        runtime
            .block_on(async { timeout(GUARD, restore_and_stop(&config, &expected)).await })
            .expect("bounded restart")
            .expect("same exact component payload after restart");
    }
}
