use super::*;
use crate::held_drop::HeldDropPlan;
use crate::inventory::{PlayerInventory, HOTBAR_START};
use ferrumc_items::{encode_persisted_stack, ComponentValue};
use ferrumc_sim::{Difficulty, GroundItemEvent, GroundItemLimits, VitalsSnapshot};

fn payload(count: u8) -> ItemStack {
    ItemStack::try_new(
        ItemId::from_name("stone").unwrap(),
        count,
        ComponentPatch::new(vec![ComponentValue::MaxStackSize(16)], vec![]),
    )
    .unwrap()
}

async fn fixture(
    items: usize,
    bytes: usize,
    resident: bool,
) -> (SessionRouter, SimShard, PlayerId) {
    let mut router = SessionRouter::new();
    let mut inbox = router.register_shard(ShardPos::new(0, 0));
    let player = PlayerId::offline("actual-held-drop");
    let _handle = router
        .join_player(player, "actual-held-drop", Vec3::new(2.5, 64.0, 3.5))
        .unwrap();
    let mut shard = test_shard();
    shard
        .initialize_entities_with_item_budget(
            NonZeroUsize::new(items).unwrap(),
            NonZeroUsize::new(bytes).unwrap(),
        )
        .unwrap();
    shard
        .enable_ground_items(
            GroundItemLimits::new(
                NonZeroUsize::new(items).unwrap(),
                NonZeroUsize::new(bytes).unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
    shard.enable_survival(Difficulty::Normal, true).unwrap();
    while let Ok(input) = inbox.try_recv() {
        shard.enqueue(input).unwrap();
    }
    if resident {
        assert_eq!(
            load_chunks(
                &mut shard,
                test_target().await.store(),
                &test_generator(),
                &[ChunkPos::new(0, 0)],
                1
            )
            .await
            .len(),
            1
        );
    }
    shard.run_tick();
    (router, shard, player)
}

async fn dispatch(
    router: &mut SessionRouter,
    shard: &mut SimShard,
    player: PlayerId,
    stack: ItemStack,
) -> bool {
    let (reply, result) = oneshot::channel();
    let (storage_tx, _storage_rx) = mpsc::channel(1);
    handle_command(
        router,
        shard,
        &test_target().await,
        &test_generator(),
        &storage_tx,
        Tick::ZERO,
        &mut 0,
        &mut WorldTime::new(),
        &mut BTreeMap::new(),
        &mut PlacementCompletions::new(),
        SimCommand::PlayerHeldDrop {
            player,
            stack,
            reply,
        },
    )
    .await
    .unwrap();
    result.await.unwrap()
}

#[tokio::test]
async fn held_drop_actual_driver_one_whole_commit_and_owned_spawn_preserve_payload() {
    for status in [3, 4] {
        let (mut router, mut shard, player) = fixture(8, 1024 * 1024, true).await;
        // Change actor position after connection inventory preparation; driver must read this real state.
        let mut slots = std::array::from_fn(|_| ItemStack::empty());
        slots[HOTBAR_START] = payload(7);
        let mut inventory =
            PlayerInventory::from_persisted(GameMode::Survival, 0, slots, ItemStack::empty());
        let state = inventory.state_id();
        let (stack, commit) = HeldDropPlan::prepare(&inventory, status)
            .unwrap()
            .unwrap()
            .into_parts();
        let expected = encode_persisted_stack(&stack).unwrap();
        let actual_position = Vec3::new(5.5, 64.0, 7.5);
        shard
            .enqueue(GameInput::PlayerMove {
                player,
                position: Some(actual_position),
                yaw: Some(90.0),
                pitch: Some(30.0),
            })
            .unwrap();
        shard.run_tick();
        assert!(dispatch(&mut router, &mut shard, player, stack).await);
        assert_eq!(inventory.slot(HOTBAR_START).unwrap().count(), 7);
        assert_eq!(inventory.state_id(), state);
        commit.commit(&mut inventory).unwrap();
        assert_eq!(
            inventory.slot(HOTBAR_START).unwrap().count(),
            if status == 3 { 0 } else { 6 }
        );
        assert!(shard.ground_item_snapshots(None, 8).unwrap().is_empty());
        crate::ground_runtime::GroundRuntime::refill(&mut router, &mut shard).unwrap();
        shard.run_tick();
        let spawn = shard
            .take_ground_item_events(8)
            .unwrap()
            .into_iter()
            .find_map(|event| match event {
                GroundItemEvent::Spawn { snapshot } => Some(snapshot),
                _ => None,
            })
            .unwrap();
        assert_eq!(spawn.base().transform().position(), actual_position);
        assert_eq!(
            encode_persisted_stack(spawn.item_state().stack()).unwrap(),
            expected
        );
        assert_eq!(
            spawn.item_state().properties().thrower(),
            Some(ferrumc_core::EntityUuid::from_uuid(player.as_uuid()))
        );
        assert_eq!(spawn.item_state().properties().owner_target(), None);
        assert_eq!(
            spawn.item_state().properties().counters().pickup_delay(),
            40
        );
        assert_eq!(shard.ground_item_snapshots(None, 8).unwrap().len(), 1);
    }
}

#[tokio::test]
async fn held_drop_actual_driver_refuses_missing_dead_spectator_and_unloaded_actors() {
    for case in 0..4 {
        let (mut router, mut shard, player) = fixture(8, 1024 * 1024, case != 3).await;
        let actor = if case == 0 {
            PlayerId::offline("absent-dropper")
        } else {
            player
        };
        if case == 1 {
            shard
                .enqueue(GameInput::RestoreVitals {
                    player,
                    snapshot: VitalsSnapshot::new(0.0, 20, 5.0, 0.0).unwrap(),
                })
                .unwrap();
        }
        if case == 2 {
            shard
                .enqueue(GameInput::SetGameMode {
                    player,
                    mode: GameMode::Spectator,
                })
                .unwrap();
        }
        shard.run_tick();
        assert!(
            !dispatch(&mut router, &mut shard, actor, payload(1)).await,
            "case {case}"
        );
        assert!(shard.ground_item_snapshots(None, 8).unwrap().is_empty());
    }
}

#[tokio::test]
async fn held_drop_actual_driver_pending_slot_and_byte_capacity_refusal_preserve_inventory() {
    for (items, bytes, fill) in [(1, 1024 * 1024, true), (8, 1, false)] {
        let (mut router, mut shard, player) = fixture(items, bytes, true).await;
        if fill {
            assert!(dispatch(&mut router, &mut shard, player, payload(1)).await);
        }
        let mut slots = std::array::from_fn(|_| ItemStack::empty());
        slots[HOTBAR_START] = payload(7);
        let inventory =
            PlayerInventory::from_persisted(GameMode::Survival, 0, slots, ItemStack::empty());
        let before = encode_persisted_stack(inventory.slot(HOTBAR_START).unwrap()).unwrap();
        let (stack, commit) = HeldDropPlan::prepare(&inventory, 4)
            .unwrap()
            .unwrap()
            .into_parts();
        assert!(!dispatch(&mut router, &mut shard, player, stack).await);
        assert!(commit.matches(&inventory).unwrap());
        assert_eq!(
            encode_persisted_stack(inventory.slot(HOTBAR_START).unwrap()).unwrap(),
            before
        );
        assert!(shard.ground_item_snapshots(None, 8).unwrap().is_empty());
    }
}
