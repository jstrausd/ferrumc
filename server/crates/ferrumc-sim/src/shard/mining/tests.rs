use std::{collections::BTreeMap, num::NonZeroUsize};

use ferrumc_core::{EntityId, EntityUuid};
use ferrumc_items::{ComponentPatch, ItemId, ItemStack};
use ferrumc_math::{ChunkPos, ShardPos};
use ferrumc_registry::block_state::{block_default_state, compute_state_id};
use ferrumc_storage::InMemoryStore;
use ferrumc_world::FlatWorldGenerator;

use super::*;
use crate::{
    ChunkTicket, GameInput, GroundItemDrop, GroundItemEvent, GroundItemIdentity, GroundItemLimits,
    GroundItemPickupResult, TicketReason,
};

async fn fixture() -> (SimShard, PlayerId) {
    let mut shard = SimShard::new(ShardPos::new(0, 0));
    shard
        .loaded_chunks_mut()
        .acquire(
            &InMemoryStore::new(),
            &FlatWorldGenerator::new(),
            ChunkPos::new(0, 0),
            ChunkTicket::of(TicketReason::Player),
        )
        .await
        .expect("resident fixture");
    let player = PlayerId::offline("mining-correction-owner");
    shard
        .enqueue(GameInput::PlayerJoin {
            player,
            position: Vec3::new(8.0, 64.0, 8.0),
        })
        .expect("join");
    let _ = shard.run_tick();
    let _ = shard.loaded_chunks_mut().take_dirty();
    (shard, player)
}

fn state(name: &str) -> BlockStateId {
    BlockStateId::new(block_default_state(name).expect("state"))
}
fn stack(name: &str, count: u8) -> ItemStack {
    ItemStack::try_new(
        ItemId::from_name(name).expect("item"),
        count,
        ComponentPatch::empty(),
    )
    .expect("stack")
}
fn tool(name: &str) -> MiningTool {
    MiningTool::new(
        Some(ItemId::from_name(name).expect("tool").id()),
        true,
        false,
    )
    .expect("context")
}
fn set(shard: &mut SimShard, position: BlockPos, state: BlockStateId) {
    assert!(matches!(
        shard.apply_block_edit(MutationCause::Test, position, state),
        MutationResult::Applied { .. }
    ));
}
fn pair(shard: &mut SimShard, name: &str) -> (BlockPos, BlockPos) {
    let lower = BlockPos::new(9, 64, 8);
    let upper = BlockPos::new(9, 65, 8);
    for (position, half) in [(lower, "lower"), (upper, "upper")] {
        set(
            shard,
            position,
            BlockStateId::new(
                compute_state_id(name, &BTreeMap::from([("half", half)])).expect("plant half"),
            ),
        );
    }
    let _ = shard.loaded_chunks_mut().take_dirty();
    (lower, upper)
}
fn ground(shard: &mut SimShard, slots: usize) {
    let slots = NonZeroUsize::new(slots).expect("slots");
    let bytes = NonZeroUsize::new(65_536).expect("bytes");
    shard
        .initialize_entities_with_item_budget(slots, bytes)
        .expect("entity owner");
    shard
        .enable_ground_items(GroundItemLimits::new(slots, bytes).expect("limits"))
        .expect("ground owner");
}
fn identity(id: u8) -> GroundItemIdentity {
    GroundItemIdentity::new(
        EntityUuid::from_bytes([id; 16]),
        EntityId::new(i32::from(id) + 100),
    )
    .expect("identity")
}
fn mine(
    shard: &mut SimShard,
    player: PlayerId,
    position: BlockPos,
    tool: MiningTool,
) -> Vec<GameOutput> {
    shard
        .enqueue(GameInput::MiningStart {
            player,
            position,
            sequence: 7,
            tool,
        })
        .expect("start");
    shard.run_tick()
}

#[tokio::test]
async fn mining_owner_sheared_double_plants_remove_both_halves_and_admit_one_exact_stack() {
    for (name, expected) in [("tall_grass", "short_grass"), ("large_fern", "fern")] {
        for upper_first in [false, true] {
            let (mut shard, player) = fixture().await;
            let (lower, upper) = pair(&mut shard, name);
            ground(&mut shard, 1);
            shard
                .refill_ground_item_identities(vec![identity(1)])
                .expect("refill");
            let initial_random = shard.mining_random;
            let outputs = mine(
                &mut shard,
                player,
                if upper_first { upper } else { lower },
                tool("shears").with_environment(false, true).with_token(71),
            );
            assert!(outputs.iter().any(|output| matches!(output, GameOutput::SurvivalBlockBroken{tool_token:71,tool_damage:1,drops,..} if drops.is_empty())));
            assert!(shard.authoritative_state(lower).is_air());
            assert!(shard.authoritative_state(upper).is_air());
            assert_eq!(
                shard.mining_random, initial_random,
                "shears bypass the chance alternative"
            );
            let snapshots = shard
                .ground_item_snapshots(None, 1)
                .expect("actual ground payload");
            assert_eq!(snapshots.len(), 1);
            assert_eq!(snapshots[0].item_state().stack(), &stack(expected, 2));
            let rows = shard.take_mutations();
            assert_eq!(rows.len(), 2);
            assert!(rows.iter().any(|row| row.position() == lower));
            assert!(rows.iter().any(|row| row.position() == upper));
            assert!(shard.loaded_chunks().has_persist_dirty());
            assert_eq!(
                shard
                    .capture_ground_item_records()
                    .expect("real capture")
                    .len(),
                1
            );
            let outputs = mine(
                &mut shard,
                player,
                if upper_first { lower } else { upper },
                tool("shears").with_token(72),
            );
            assert!(!outputs
                .iter()
                .any(|output| matches!(output, GameOutput::SurvivalBlockBroken { .. })));
            assert!(outputs
                .iter()
                .any(|output| matches!(output, GameOutput::MiningReleased { tool_token: 72, .. })));
            assert_eq!(
                shard
                    .ground_item_snapshots(None, 1)
                    .expect("remaining payload")[0]
                    .item_state()
                    .stack(),
                &stack(expected, 2)
            );
            assert_eq!(shard.take_mutations(), Vec::new());
        }
    }
}

#[tokio::test]
async fn mining_owner_real_seed_rolls_and_malformed_pairs_do_not_mint_grass() {
    for (random, seeds) in [(1, 0), (8, 1)] {
        for name in ["short_grass", "fern"] {
            let (mut shard, player) = fixture().await;
            let position = BlockPos::new(9, 64, 8);
            set(&mut shard, position, state(name));
            shard.mining_random = random;
            let outputs = mine(
                &mut shard,
                player,
                position,
                MiningTool::empty()
                    .with_environment(false, true)
                    .with_token(73),
            );
            let drops = outputs
                .iter()
                .find_map(|output| {
                    if let GameOutput::SurvivalBlockBroken {
                        drops, tool_damage, ..
                    } = output
                    {
                        assert_eq!(*tool_damage, 0);
                        Some(drops)
                    } else {
                        None
                    }
                })
                .expect("instant ordinary plant break");
            assert_eq!(drops.len(), seeds);
            if seeds == 1 {
                assert_eq!(drops[0], stack("wheat_seeds", 1));
            }
            assert_ne!(
                shard.mining_random, random,
                "a real chance roll commits with the edit"
            );
        }
    }
    for upper_first in [false, true] {
        let (mut shard, player) = fixture().await;
        let (lower, upper) = pair(&mut shard, "tall_grass");
        let retained = if upper_first { lower } else { upper };
        set(&mut shard, retained, state("stone"));
        shard.mining_random = 8;
        let outputs = mine(
            &mut shard,
            player,
            if upper_first { upper } else { lower },
            tool("shears"),
        );
        let drops = outputs
            .iter()
            .find_map(|output| {
                if let GameOutput::SurvivalBlockBroken { drops, .. } = output {
                    Some(drops)
                } else {
                    None
                }
            })
            .expect("lone half removal");
        assert_eq!(drops.as_slice(), []);
        assert_eq!(shard.authoritative_state(retained), state("stone"));
        assert_eq!(
            shard.mining_random, 8,
            "failed original pair condition makes no chance roll"
        );
    }
}

#[tokio::test]
async fn mining_owner_full_ground_retains_whole_pair_and_rng_then_retries_same_seed_once() {
    let (mut shard, player) = fixture().await;
    let (lower, upper) = pair(&mut shard, "tall_grass");
    let before = [
        shard.authoritative_state(lower),
        shard.authoritative_state(upper),
    ];
    ground(&mut shard, 1);
    shard
        .enqueue_ground_item_drop(GroundItemDrop::new(
            Vec3::new(8.0, 64.0, 8.0),
            stack("diamond", 1),
            0,
            None,
            None,
        ))
        .expect("occupied credit");
    shard.mining_random = 8;
    let outputs = mine(
        &mut shard,
        player,
        upper,
        MiningTool::empty().with_token(74),
    );
    assert!(!outputs
        .iter()
        .any(|output| matches!(output, GameOutput::SurvivalBlockBroken { .. })));
    assert!(outputs
        .iter()
        .any(|output| matches!(output, GameOutput::MiningReleased { tool_token: 74, .. })));
    assert_eq!(
        [
            shard.authoritative_state(lower),
            shard.authoritative_state(upper)
        ],
        before
    );
    assert_eq!(shard.mining_random, 8);
    assert_eq!(shard.take_mutations(), Vec::new());
    assert!(!shard.loaded_chunks().has_persist_dirty());
    assert_eq!(shard.loaded_chunks_mut().take_dirty(), Vec::new());
    shard
        .refill_ground_item_identities(vec![identity(1)])
        .expect("actual spawn credit");
    let _ = shard.run_tick();
    let ticket = shard
        .take_ground_item_events(256)
        .expect("actual offered item")
        .into_iter()
        .find_map(|event| {
            if let GroundItemEvent::PickupOffer { ticket, .. } = event {
                Some(ticket)
            } else {
                None
            }
        })
        .expect("move-owned pickup offer");
    shard
        .enqueue_ground_item_pickup_result(GroundItemPickupResult::new(ticket, ItemStack::empty()))
        .expect("real complete deposit receipt");
    let _ = shard.run_tick();
    let _ = shard.take_ground_item_events(256).expect("settled receipt");
    shard
        .refill_ground_item_identities(vec![identity(2)])
        .expect("seed spawn identity");
    let outputs = mine(
        &mut shard,
        player,
        lower,
        MiningTool::empty().with_token(75),
    );
    assert!(outputs.iter().any(|output| matches!(
        output,
        GameOutput::SurvivalBlockBroken { tool_token: 75, .. }
    )));
    assert!(shard.authoritative_state(lower).is_air());
    assert!(shard.authoritative_state(upper).is_air());
    assert_ne!(shard.mining_random, 8);
    assert_eq!(
        shard
            .ground_item_snapshots(None, 1)
            .expect("one seed entity")[0]
            .item_state()
            .stack(),
        &stack("wheat_seeds", 1)
    );
}

#[tokio::test]
async fn mining_owner_full_water_schedule_refuses_both_plant_halves_without_rng_or_dirty_change() {
    let (mut shard, player) = fixture().await;
    let (lower, upper) = pair(&mut shard, "tall_grass");
    let before = [
        shard.authoritative_state(lower),
        shard.authoritative_state(upper),
    ];
    let occupied = BlockPos::new(12, 65, 8);
    set(&mut shard, occupied, state("water"));
    set(&mut shard, BlockPos::new(10, 65, 8), state("water"));
    let _ = shard.loaded_chunks_mut().take_dirty();
    shard.enable_water(1, true).expect("bounded water owner");
    shard
        .enqueue(GameInput::WaterSchedule { position: occupied })
        .expect("fill queue");
    shard.mining_random = 8;
    let outputs = mine(
        &mut shard,
        player,
        upper,
        MiningTool::empty().with_token(76),
    );
    assert!(outputs
        .iter()
        .any(|output| matches!(output, GameOutput::MiningReleased { tool_token: 76, .. })));
    assert!(!outputs
        .iter()
        .any(|output| matches!(output, GameOutput::SurvivalBlockBroken { .. })));
    assert_eq!(
        [
            shard.authoritative_state(lower),
            shard.authoritative_state(upper)
        ],
        before
    );
    assert_eq!(shard.mining_random, 8);
    assert_eq!(shard.water_queue_len(), Some(1));
    assert_eq!(shard.take_mutations(), Vec::new());
    assert!(!shard.loaded_chunks().has_persist_dirty());
    assert_eq!(shard.loaded_chunks_mut().take_dirty(), Vec::new());
}

#[tokio::test]
async fn mining_owner_dynamic_ground_and_real_resident_eye_water_preserve_elapsed_tool_witness() {
    let (mut shard, player) = fixture().await;
    let position = BlockPos::new(9, 64, 8);
    let eye = BlockPos::new(8, 65, 8);
    set(&mut shard, position, state("stone"));
    set(&mut shard, eye, state("water"));
    shard.enable_water(4096, true).expect("actual fluid facts");
    let tool = tool("wooden_pickaxe").with_token(77);
    let _ = mine(&mut shard, player, position, tool);
    for _ in 0..11 {
        let _ = shard.run_tick();
    }
    let before = shard.mining_work[&player];
    assert_eq!(
        before.stage, 1,
        "eye at source water has the actual underwater multiplier"
    );
    set(&mut shard, eye, BlockStateId::AIR);
    let outputs = shard.run_tick();
    let dry = shard.mining_work[&player];
    assert_eq!(dry.elapsed, before.elapsed + 1);
    assert_eq!(dry.tool.token(), 77);
    assert!(dry.stage >= 5);
    assert!(!outputs
        .iter()
        .any(|output| matches!(output, GameOutput::MiningReleased { .. })));
    shard
        .enqueue(GameInput::MiningEnvironmentChanged {
            player,
            on_ground: false,
        })
        .expect("genuine movement flag");
    let _ = shard.run_tick();
    let airborne = shard.mining_work[&player];
    assert_eq!(airborne.elapsed, dry.elapsed + 1);
    assert_eq!(airborne.tool.token(), 77);
    assert!(
        airborne.stage < dry.stage,
        "original current environment scales total elapsed progress"
    );
    shard
        .enqueue(GameInput::MiningEnvironmentChanged {
            player,
            on_ground: true,
        })
        .expect("ground change");
    shard
        .enqueue(GameInput::MiningFinish {
            player,
            position,
            sequence: 8,
            tool,
        })
        .expect("same exact tool finish");
    let mut outputs = shard.run_tick();
    for _ in 0..30 {
        if shard.authoritative_state(position).is_air() {
            break;
        }
        outputs.extend(shard.run_tick());
    }
    assert!(shard.authoritative_state(position).is_air());
    assert_eq!(
        outputs
            .iter()
            .filter(|output| matches!(
                output,
                GameOutput::SurvivalBlockBroken {
                    tool_token: 77,
                    tool_damage: 1,
                    ..
                }
            ))
            .count(),
        1
    );
    assert!(!outputs
        .iter()
        .any(|output| matches!(output, GameOutput::MiningReleased { tool_token: 77, .. })));
}
