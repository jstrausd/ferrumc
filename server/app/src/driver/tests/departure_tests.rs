//! Genuine driver teardown, current-inventory save and fresh-incarnation checks.

use super::*;
use ferrumc_sim::{Difficulty, VitalsSnapshot};
use ferrumc_storage::PlayerStore;

async fn read_vitals(
    commands: &mpsc::Sender<SimCommand>,
    player: PlayerId,
) -> Option<VitalsSnapshot> {
    let (reply, received) = oneshot::channel();
    commands
        .send(SimCommand::ReadVitals { player, reply })
        .await
        .unwrap();
    received.await.unwrap()
}

async fn register(commands: &mpsc::Sender<SimCommand>, player: PlayerId) {
    let (effects, _receiver) = mpsc::channel(4);
    let (reply, accepted) = oneshot::channel();
    commands
        .send(SimCommand::RegisterGameplay {
            player,
            sender: effects,
            reply,
        })
        .await
        .unwrap();
    assert!(accepted.await.unwrap());
}

async fn save_and_restore_current_inventory(
    store: &InMemoryStore,
    player: PlayerId,
    observed: VitalsSnapshot,
) {
    let mut inventory = crate::inventory::PlayerInventory::from_persisted(
        GameMode::Survival,
        0,
        std::array::from_fn(|_| ItemStack::empty()),
        ItemStack::empty(),
    );
    inventory.set_creative_slot(
        36,
        ItemStack::try_new(
            ItemId::from_name("diamond").unwrap(),
            13,
            ComponentPatch::empty(),
        )
        .unwrap(),
    );
    let mut data = crate::player_data::PlayerData::capture(spawn(), 0.0, 0.0, &inventory);
    data.set_vitals(observed).unwrap();
    store
        .save_player(player, data.to_record(GameMode::Survival).unwrap())
        .await
        .unwrap();
    let restored = crate::player_data::PlayerData::from_record(
        &store.load_player(player).await.unwrap().unwrap(),
    )
    .unwrap();
    assert_eq!(restored.vitals(), observed);
    assert_eq!(
        restored.restore_inventory(GameMode::Survival).held(),
        inventory.held()
    );
}

#[tokio::test(start_paused = true)]
async fn forced_session_leave_saves_current_inventory_and_all_vitals_then_retires_observation() {
    let store = Arc::new(test_store().await);
    let mut shard = test_shard();
    shard.enable_survival(Difficulty::Normal, true).unwrap();
    let mut router = SessionRouter::with_capacities(32, 2);
    let shard_rx = router.register_shard(ShardPos::new(0, 0));
    let (commands, receiver) = mpsc::channel(4);
    let (storage, _storage_rx) = mpsc::channel(4);
    let (stop, stopped) = watch::channel(false);
    let (fatal, fatal_rx) = watch::channel(false);
    let driver = tokio::spawn(super::super::run(
        router,
        shard,
        crate::persistence::PersistenceTarget::verify(
            store.clone(),
            crate::persistence::fixtures::binding(),
        )
        .await
        .unwrap(),
        test_generator(),
        None,
        shard_rx,
        receiver,
        Duration::from_secs(1),
        Arc::new(CounterRegistry::new()),
        ServerClock::new(),
        storage,
        SnapshotPublisher::default(),
        Arc::new(NetTelemetryHub::new()),
        Arc::new(BlockEventDispatcher::new(PluginHost::in_memory())),
        fatal,
        stopped,
    ));
    let player = PlayerId::offline("forced-save");
    let (reply, joined) = oneshot::channel();
    commands
        .send(SimCommand::Join {
            player,
            name: "forced-save".to_owned(),
            position: spawn(),
            equipment: Vec::new(),
            reply,
        })
        .await
        .unwrap();
    let _undrained_handle = joined.await.unwrap().unwrap();
    register(&commands, player).await;
    tokio::time::advance(Duration::from_secs(1)).await;
    assert!(read_vitals(&commands, player).await.is_some());

    // Hunger eighteen advances the ordinary regeneration timer without
    // reaching its eighty-tick heal threshold in this boundary.
    let last = VitalsSnapshot::new(6.0, 18, 2.0, 3.0)
        .unwrap()
        .with_timers(37, 9, 5, 4.0, 13)
        .unwrap();
    // The genuine two-slot Session queue still holds join time. Two actual
    // health publications in one boundary therefore force its slow-reader leave.
    for snapshot in [VitalsSnapshot::new(7.0, 18, 2.0, 3.0).unwrap(), last] {
        let (reply, admitted) = oneshot::channel();
        commands
            .send(SimCommand::GameplayInput {
                player,
                input: GameInput::RestoreVitals { player, snapshot },
                acceptance: Some(reply),
            })
            .await
            .unwrap();
        admitted.await.unwrap().unwrap();
    }
    tokio::time::advance(Duration::from_secs(1)).await;
    let (reply, unregistered) = oneshot::channel();
    commands
        .send(SimCommand::UnregisterGameplay { player, reply })
        .await
        .unwrap();
    unregistered.await.unwrap();
    // Unregister runs the admitted leave prefix before removing the producer.
    let observed = read_vitals(&commands, player).await.unwrap();
    assert_eq!(observed.health(), last.health());
    assert_eq!(observed.food(), last.food());
    assert_eq!(observed.saturation(), last.saturation());
    assert_eq!(observed.exhaustion(), last.exhaustion());
    assert_eq!(observed.food_tick_timer(), 38);
    assert_eq!(observed.invulnerable_ticks(), 8);
    assert_eq!(observed.hurt_ticks(), 4);
    assert_eq!(observed.last_hurt(), 4.0);
    assert_eq!(observed.peaceful_phase(), 14);

    save_and_restore_current_inventory(&store, player, observed).await;

    let (reply, left) = oneshot::channel();
    commands
        .send(SimCommand::Event {
            event: NetEvent::disconnected(player, DisconnectReason::OutboundOverflow),
            acceptance: Some(reply),
        })
        .await
        .unwrap();
    assert!(matches!(
        left.await.unwrap(),
        Err(SessionError::UnknownPlayer { .. })
    ));
    assert!(read_vitals(&commands, player).await.is_none());
    stop.send(true).unwrap();
    driver.await.unwrap().unwrap();
    assert!(!*fatal_rx.borrow());
}
