//! Unreadable player records must reject Play admission without being replaced.
//!
//! Each case drives the public server over a real socket, then gracefully shuts
//! it down before inspecting the durable record. Before the fix, all three loads
//! became fresh-player defaults and the shared leave teardown overwrote the
//! original row.

mod common;

use std::path::{Path, PathBuf};
use std::time::Duration;

use redb::{Database, TableDefinition};
use tokio::runtime::{Builder, Runtime};
use tokio::time::timeout;

use ferrumc_core::{GameMode, PlayerId, ServerError};
use ferrumc_proto::generated::play::ClientboundPlayPacket;
use ferrumc_storage::WorldStore;
use ferrumc_storage::{PlayerRecord, PlayerStore, RedbStore, SchemaVersion};

use ferrumc_app::AppConfig;

use common::{login_to_play, TestClient};

/// Overall guard so a rejected connection or shutdown regression cannot hang.
const GUARD: Duration = Duration::from_secs(10);

/// Current app-owned player payload schema.
const PLAYER_SCHEMA_VERSION: SchemaVersion = SchemaVersion::new(5);
const JSON_PLAYER_SCHEMA_VERSION: SchemaVersion = SchemaVersion::new(2);

/// Raw player table used only to preserve and inspect an undecodable backend row.
const PLAYER_TABLE: TableDefinition<'_, &[u8], &[u8]> = TableDefinition::new("ferrumc:player");

/// Builds a durable server on an ephemeral port with one spawn chunk.
fn persistent_config(world_dir: &Path) -> AppConfig {
    AppConfig::from_toml_str(
        "bind = \"127.0.0.1:0\"\n\
         spawn_chunk_radius = 0\n\
         view_distance = 0\n",
    )
    .expect("player-load-failure config parses")
    .with_world_dir(Some(world_dir.to_path_buf()))
    .expect("world directory preserves a valid config")
}

/// Returns the database path selected by the app for `world_dir`.
fn database_path(world_dir: &Path) -> PathBuf {
    world_dir.join("world.redb")
}

/// Creates an isolated world directory under the repository-owned scratch root.
fn temp_world() -> tempfile::TempDir {
    let scratch = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(".codex-tmp");
    std::fs::create_dir_all(&scratch).expect("create repository scratch directory");
    tempfile::Builder::new()
        .prefix("packet24-player-load-")
        .tempdir_in(scratch)
        .expect("create test-local world directory")
}

/// Builds the multi-thread runtime used only for async server/store operations.
fn test_runtime() -> Runtime {
    Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("build test runtime")
}

/// A valid current-layout payload with state distinct from spawn defaults.
fn valid_payload() -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "x":21.5,"y":70.0,"z":9.25,"yaw":45.0,"pitch":-12.0,"selected_slot":2,
        "slots":vec![serde_json::json!({"item_id":0,"count":0});46],
        "carried":{"item_id":0,"count":0},
    }))
    .expect("valid schema-two payload")
}

/// Reads through the final spawn-chunk frame so a pre-fix admitted connection
/// has completed its fallible join-kit writes before the socket is closed.
async fn finish_join_kit(client: &mut TestClient) -> anyhow::Result<()> {
    loop {
        if matches!(
            client.next_play().await?,
            ClientboundPlayPacket::ChunkDataAndLight(_)
        ) {
            return Ok(());
        }
    }
}

/// Attempts one login, drains any accidentally admitted session, and shuts down.
async fn attempt_login_and_shutdown(config: &AppConfig, name: &str) -> bool {
    let server = ferrumc_app::run(config).await.expect("server starts");
    let login = timeout(GUARD, login_to_play(server.local_addr(), name))
        .await
        .expect("login attempt finishes within guard");
    let admitted = match login {
        Ok(mut client) => {
            timeout(GUARD, finish_join_kit(&mut client))
                .await
                .expect("join kit finishes within guard")
                .expect("join kit reaches its spawn chunk");
            drop(client);
            true
        }
        Err(_) => false,
    };

    timeout(GUARD, server.shutdown())
        .await
        .expect("shutdown finishes within guard")
        .expect("server shuts down cleanly");
    admitted
}

/// Seeds one typed record without starting the app.
fn seed_player(runtime: &Runtime, path: &Path, player: PlayerId, record: PlayerRecord) {
    let store = RedbStore::open_with_generation_metadata(path).expect("open player store");
    runtime
        .block_on(store.ensure_generation_identity(
            ferrumc_storage::GenerationKey::new(
                ferrumc_core::WorldId::new(0),
                ferrumc_core::DimensionId::new(0),
            ),
            ferrumc_world::GenerationIdentity::flat_overworld(ferrumc_world::WorldSeed::new(0)),
        ))
        .expect("initialize fixture identity");
    runtime
        .block_on(store.save_player(player, record))
        .expect("seed player record");
}

/// Loads one typed record after the app has fully released the database.
fn load_player(runtime: &Runtime, path: &Path, player: PlayerId) -> PlayerRecord {
    let store = RedbStore::open_with_generation_metadata(path).expect("reopen player store");
    runtime
        .block_on(store.load_player(player))
        .expect("load player record")
        .expect("player record remains present")
}

/// Writes an intentionally malformed storage envelope directly into the
/// test-local player table.
fn insert_raw_player(path: &Path, player: PlayerId, value: &[u8]) {
    let database = Database::create(path).expect("open raw test database");
    let transaction = database.begin_write().expect("begin raw write");
    {
        let mut table = transaction
            .open_table(PLAYER_TABLE)
            .expect("open raw player table");
        let key = *player.as_uuid().as_bytes();
        table
            .insert(key.as_slice(), value)
            .expect("insert malformed player row");
    }
    transaction.commit().expect("commit malformed player row");
}

/// Reads exact storage-envelope bytes without asking the typed codec to decode
/// the deliberately malformed value.
fn raw_player(path: &Path, player: PlayerId) -> Vec<u8> {
    let database = Database::create(path).expect("open raw test database");
    let transaction = database.begin_read().expect("begin raw read");
    let table = transaction
        .open_table(PLAYER_TABLE)
        .expect("open raw player table");
    let key = *player.as_uuid().as_bytes();
    let guard = table
        .get(key.as_slice())
        .expect("read raw player row")
        .expect("raw player row remains present");
    guard.value().to_vec()
}

/// Runs one app-payload failure and compares the complete typed record afterward.
fn assert_typed_record_rejected_unchanged(
    runtime: &Runtime,
    world_dir: &Path,
    name: &str,
    original: &PlayerRecord,
) {
    let path = database_path(world_dir);
    let player = PlayerId::offline(name);
    seed_player(runtime, &path, player, original.clone());
    let original_raw = raw_player(&path, player);

    let admitted = runtime.block_on(attempt_login_and_shutdown(
        &persistent_config(world_dir),
        name,
    ));
    let committed = load_player(runtime, &path, player);
    assert_eq!(
        raw_player(&path, player),
        original_raw,
        "the complete persisted record bytes must remain unchanged",
    );
    assert_eq!(
        committed.data(),
        original.data(),
        "the original payload bytes must remain unchanged",
    );
    assert_eq!(
        &committed, original,
        "schema, game mode, and payload must remain byte-for-byte equivalent",
    );
    assert!(
        !admitted,
        "an unreadable player record must be rejected before JoinGame",
    );
}

#[test]
fn backend_load_failure_does_not_overwrite_the_original_record() {
    let temp = temp_world();
    let path = database_path(temp.path());
    let player = PlayerId::offline("BackendFault");
    let runtime = test_runtime();

    // Initialize the store format/tables through the shipping backend, then
    // replace this player's value with a truncated envelope. `RedbStore` returns
    // a classified backend error because even schema + game mode cannot be read.
    let store = RedbStore::open_with_generation_metadata(&path).expect("initialize player store");
    runtime
        .block_on(store.ensure_generation_identity(
            ferrumc_storage::GenerationKey::new(
                ferrumc_core::WorldId::new(0),
                ferrumc_core::DimensionId::new(0),
            ),
            ferrumc_world::GenerationIdentity::flat_overworld(ferrumc_world::WorldSeed::new(0)),
        ))
        .expect("initialize fixture identity");
    drop(store);
    let original = vec![0x00, 0x00, 0x00, 0x01];
    insert_raw_player(&path, player, &original);

    let store = RedbStore::open_with_generation_metadata(&path).expect("reopen typed player store");
    let error = runtime
        .block_on(store.load_player(player))
        .expect_err("the malformed envelope must fail typed loading");
    assert!(matches!(error, ServerError::Internal { .. }));
    drop(store);

    let admitted = runtime.block_on(attempt_login_and_shutdown(
        &persistent_config(temp.path()),
        "BackendFault",
    ));
    assert_eq!(
        raw_player(&path, player),
        original,
        "the malformed backend row must remain byte-for-byte unchanged",
    );
    assert!(
        !admitted,
        "a backend load error must be rejected before JoinGame",
    );
}

#[test]
fn corrupt_player_payload_does_not_overwrite_the_original_record() {
    let temp = temp_world();
    let runtime = test_runtime();
    let original = PlayerRecord::new(
        PLAYER_SCHEMA_VERSION,
        GameMode::Survival,
        vec![0xff, 0x00, b'{'],
    )
    .expect("bounded corrupt record");
    assert_typed_record_rejected_unchanged(&runtime, temp.path(), "CorruptPayload", &original);
}

#[test]
fn malformed_schema_two_carried_does_not_overwrite_the_original_record() {
    let runtime = test_runtime();
    for (name, carried) in [
        ("MissingCursor", None),
        (
            "InvalidCursor",
            Some(serde_json::json!({"item_id":1,"count":65})),
        ),
        (
            "UnknownCursor",
            Some(serde_json::json!({"item_id":i32::MAX,"count":7})),
        ),
    ] {
        let temp = temp_world();
        let mut payload: serde_json::Value =
            serde_json::from_slice(&valid_payload()).expect("valid payload");
        if let Some(carried) = carried {
            payload["carried"] = carried;
        } else {
            payload.as_object_mut().expect("object").remove("carried");
        }
        let original = PlayerRecord::new(
            JSON_PLAYER_SCHEMA_VERSION,
            GameMode::Survival,
            serde_json::to_vec(&payload).expect("JSON"),
        )
        .expect("bounded record");
        assert_typed_record_rejected_unchanged(&runtime, temp.path(), name, &original);
    }
}

fn valid_binary_payload() -> Vec<u8> {
    let mut out = b"FCPS".to_vec();
    out.extend_from_slice(&3_u16.to_be_bytes());
    for value in [21.5_f64, 70.0, 9.25] {
        out.extend_from_slice(&value.to_be_bytes());
    }
    for value in [45.0_f32, -12.0] {
        out.extend_from_slice(&value.to_be_bytes());
    }
    out.push(2);
    out.extend_from_slice(&46_u16.to_be_bytes());
    for _ in 0..47 {
        out.extend_from_slice(&7_u32.to_be_bytes());
        out.extend_from_slice(b"FCIS\x00\x01\x00");
    }
    out
}

#[test]
fn malformed_schema_three_state_and_item_frames_preserve_original_bytes() {
    let runtime = test_runtime();
    for (name, offset, replacement) in [
        ("UnsafePosition", 6, f64::NAN.to_be_bytes().to_vec()),
        ("InvalidSelection", 38, vec![9]),
        ("InvalidLayout", 39, 45_u16.to_be_bytes().to_vec()),
        ("OversizedFrame", 41, u32::MAX.to_be_bytes().to_vec()),
        ("BadItemMagic", 45, b"NOPE".to_vec()),
        ("InvalidCarriedKind", 557, vec![2]),
    ] {
        let temp = temp_world();
        let mut payload = valid_binary_payload();
        payload[offset..offset + replacement.len()].copy_from_slice(&replacement);
        let original = PlayerRecord::new(SchemaVersion::new(3), GameMode::Survival, payload)
            .expect("bounded malformed record");
        assert_typed_record_rejected_unchanged(&runtime, temp.path(), name, &original);
    }
}

#[test]
fn incompatible_player_schema_does_not_overwrite_the_original_record() {
    let temp = temp_world();
    let runtime = test_runtime();
    let original = PlayerRecord::new(
        SchemaVersion::new(9999),
        GameMode::Spectator,
        valid_payload(),
    )
    .expect("bounded incompatible record");
    assert_typed_record_rejected_unchanged(&runtime, temp.path(), "FutureSchema", &original);
}

#[test]
fn confirmed_missing_player_still_admits_and_saves() {
    let temp = temp_world();
    let runtime = test_runtime();
    let name = "ConfirmedMissing";
    let player = PlayerId::offline(name);

    let admitted = runtime.block_on(attempt_login_and_shutdown(
        &persistent_config(temp.path()),
        name,
    ));
    assert!(
        admitted,
        "a confirmed missing record should admit a fresh player",
    );
    let committed = load_player(&runtime, &database_path(temp.path()), player);
    assert_eq!(committed.schema_version(), PLAYER_SCHEMA_VERSION);
    assert_eq!(committed.game_mode(), GameMode::Survival);
}
