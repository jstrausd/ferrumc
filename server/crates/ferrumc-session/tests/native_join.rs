//! NET-04.2e1: limits, registry layout, native registration, reservation,
//! admission and cancel with genuine memory and redb admissions.

mod native_support;

use ferrumc_core::{DimensionId, GameMode, WorldId};
use ferrumc_math::{ShardPos, Vec3};
use ferrumc_net::{Criticality, OutboundPriority};
use ferrumc_proto::generated::configuration::{KnownPack, RegistryData, RegistryEntry};
use ferrumc_proto::generated::play::{ClientboundPlayPacket, JoinGame};
use ferrumc_session::{
    CollectorSessionError, CollectorSessionLimits, NativeEndpointState, NativeSessionPhase,
    SessionError, SessionRouter, MAX_BASELINES_PER_VIEWER, MAX_COLLECTOR_AUDIENCE,
    MAX_COLLECTOR_SCRATCH, MAX_COLLECTOR_SESSIONS, MAX_COLLECTOR_WORK, MAX_TOTAL_BASELINES,
};
use ferrumc_sim::{GenerationBinding, ShardPartitioner};
use ferrumc_storage::GenerationKey;
use ferrumc_testkit::assert_wire_frame;
use ferrumc_world::{GenerationIdentity, WorldSeed};
use native_support::*;

fn limit_args(over: usize) -> [usize; 6] {
    let mut args = [8, 7, 64, 8, 4096, 65_536];
    if over < 6 {
        args = [
            MAX_COLLECTOR_SESSIONS,
            MAX_BASELINES_PER_VIEWER,
            MAX_TOTAL_BASELINES,
            MAX_COLLECTOR_AUDIENCE,
            MAX_COLLECTOR_WORK,
            MAX_COLLECTOR_SCRATCH,
        ];
        args[over] += 1;
    }
    args
}

fn try_limits(a: [usize; 6]) -> Result<CollectorSessionLimits, CollectorSessionError> {
    CollectorSessionLimits::new(nz(a[0]), nz(a[1]), nz(a[2]), nz(a[3]), nz(a[4]), nz(a[5]))
}

#[test]
fn limits_accept_hard_maxima_and_refuse_each_n_plus_one() {
    let max = [
        MAX_COLLECTOR_SESSIONS,
        MAX_BASELINES_PER_VIEWER,
        MAX_TOTAL_BASELINES,
        MAX_COLLECTOR_AUDIENCE,
        MAX_COLLECTOR_WORK,
        MAX_COLLECTOR_SCRATCH,
    ];
    let limits = try_limits(max).unwrap();
    assert_eq!(limits.sessions().get(), 1024);
    assert_eq!(limits.baselines_per_viewer().get(), 1023);
    assert_eq!(limits.total_baselines().get(), 262_144);
    assert_eq!(limits.audience().get(), 1024);
    assert_eq!(limits.work().get(), 4_194_304);
    assert_eq!(limits.scratch().get(), 1_048_576);
    let names = [
        "sessions",
        "baselines per viewer",
        "total baselines",
        "audience",
        "work",
        "scratch",
    ];
    for (over, name) in names.iter().enumerate() {
        assert_eq!(
            try_limits(limit_args(over)),
            Err(CollectorSessionError::InvalidLimit(name)),
            "{name}"
        );
    }
    // N-1 is accepted for every hard maximum.
    let below = max.map(|value| value - 1);
    assert!(try_limits([
        below[0],
        below[1] - 1,
        below[2],
        below[3],
        below[4],
        below[5]
    ])
    .is_ok());
}

#[test]
fn limits_refuse_cross_rule_violations() {
    assert_eq!(
        try_limits([4, 4, 64, 4, 64, 64]),
        Err(CollectorSessionError::InvalidLimit(
            "baselines per viewer must be below sessions"
        ))
    );
    assert!(try_limits([4, 3, 64, 4, 64, 64]).is_ok());
    assert_eq!(
        try_limits([4, 3, 64, 5, 64, 64]),
        Err(CollectorSessionError::InvalidLimit(
            "audience must not exceed sessions"
        ))
    );
    // One session cannot hold a per-viewer baseline below it.
    assert!(try_limits([1, 1, 1, 1, 1, 1]).is_err());
}

fn malformed_registries() -> Vec<(RegistryData, KnownPack, &'static str)> {
    let (registry, pack) = registries();
    let overworld = || RegistryEntry::new(bs("minecraft:overworld"), None);
    vec![
        (
            RegistryData::new(bs("minecraft:worldgen/biome"), vec![overworld()]),
            pack.clone(),
            "registry identifier",
        ),
        (
            RegistryData::new(bs("minecraft:dimension_type"), vec![]),
            pack.clone(),
            "dimension entry count",
        ),
        (
            RegistryData::new(
                bs("minecraft:dimension_type"),
                vec![
                    overworld(),
                    RegistryEntry::new(bs("minecraft:the_nether"), None),
                ],
            ),
            pack.clone(),
            "dimension entry count",
        ),
        (
            RegistryData::new(
                bs("minecraft:dimension_type"),
                vec![RegistryEntry::new(bs("minecraft:the_end"), None)],
            ),
            pack.clone(),
            "dimension entry name",
        ),
        (
            RegistryData::new(
                bs("minecraft:dimension_type"),
                vec![RegistryEntry::new(
                    bs("minecraft:overworld"),
                    Some(ferrumc_nbt::NbtTag::Byte(0)),
                )],
            ),
            pack.clone(),
            "dimension entry data",
        ),
        (
            registry.clone(),
            KnownPack::new(bs("ferrumc"), bs("core"), bs("1.21.8")),
            "core known pack",
        ),
        (
            registry.clone(),
            KnownPack::new(bs("minecraft"), bs("extra"), bs("1.21.8")),
            "core known pack",
        ),
        (
            registry,
            KnownPack::new(bs("minecraft"), bs("core"), bs("1.21.7")),
            "core known pack",
        ),
    ]
}

#[test]
fn initialization_validates_the_one_registry_layout_and_distance() {
    for (registry, pack, reason) in malformed_registries() {
        let mut router = SessionRouter::new();
        assert_eq!(
            router.initialize_collector_sessions(limits(4), &registry, &pack, 10),
            Err(CollectorSessionError::UnsupportedRegistryLayout(reason))
        );
        assert!(router.collector_session_limits().is_none());
    }
    let (registry, pack) = registries();
    for distance in [-1, 33, i32::MIN, i32::MAX] {
        assert_eq!(
            SessionRouter::new().initialize_collector_sessions(
                limits(4),
                &registry,
                &pack,
                distance
            ),
            Err(CollectorSessionError::InvalidSimulationDistance { distance })
        );
    }
    for distance in [0, 32] {
        assert!(SessionRouter::new()
            .initialize_collector_sessions(limits(4), &registry, &pack, distance)
            .is_ok());
    }
    let mut router = SessionRouter::new();
    router
        .initialize_collector_sessions(limits(4), &registry, &pack, 10)
        .unwrap();
    assert_eq!(
        router.initialize_collector_sessions(limits(8), &registry, &pack, 10),
        Err(CollectorSessionError::AlreadyInitialized)
    );
    assert_eq!(router.collector_session_limits(), Some(limits(4)));

    // Lane capacity 1024 is the hard maximum; 1025 is refused.
    assert!(SessionRouter::with_capacities(1024, 64)
        .initialize_collector_sessions(limits(4), &registry, &pack, 10)
        .is_ok());
    assert_eq!(
        SessionRouter::with_capacities(1025, 64).initialize_collector_sessions(
            limits(4),
            &registry,
            &pack,
            10
        ),
        Err(CollectorSessionError::InvalidLimit("native lane capacity"))
    );

    // Initialization happens before any session.
    let mut busy = SessionRouter::new();
    let _registration = busy.register_shard(ShardPos::new(0, 0));
    let _handle = busy.join_player(player("early"), "early", spawn()).unwrap();
    assert_eq!(
        busy.initialize_collector_sessions(limits(4), &registry, &pack, 10),
        Err(CollectorSessionError::NotEmpty)
    );
}

#[test]
fn native_registration_requires_initialization_and_the_layout_scope() {
    let mut plain = SessionRouter::new();
    assert_eq!(
        plain
            .register_native_world_shard(home(), generation())
            .err(),
        Some(CollectorSessionError::NotInitialized)
    );
    let mut router = router(16, 4, 8);
    let other_world = GenerationBinding::new(
        GenerationKey::new(WorldId::new(1), DimensionId::new(0)),
        identity(),
    );
    assert_eq!(
        router
            .register_native_world_shard(home(), other_world)
            .err(),
        Some(CollectorSessionError::Unsupported("generation"))
    );
    let other_dimension = GenerationBinding::new(
        GenerationKey::new(WorldId::new(0), DimensionId::new(1)),
        identity(),
    );
    assert_eq!(
        router
            .register_native_world_shard(home(), other_dimension)
            .err(),
        Some(CollectorSessionError::Unsupported("generation"))
    );
    let far_home =
        ShardPartitioner::for_shard(WorldId::new(0), DimensionId::new(1), ShardPos::new(0, 0))
            .unwrap();
    assert_eq!(
        router
            .register_native_world_shard(far_home, generation())
            .err(),
        Some(CollectorSessionError::Unsupported("home scope"))
    );
    // Another seed is still the supported layout; the scope binds the seed.
    let seeded = GenerationBinding::new(
        scope(),
        GenerationIdentity::flat_overworld(WorldSeed::new(1)),
    );
    let registration = router.register_native_world_shard(home(), seeded).unwrap();
    assert_eq!(registration.shard_id(), home());
    assert_eq!(
        router
            .register_native_world_shard(home(), generation())
            .err(),
        Some(CollectorSessionError::Directory(
            ferrumc_session::ShardDirectoryError::RegistrationOccupied {
                coverage: registration.lease().coverage(),
                generation: registration.lease().generation(),
            }
        ))
    );
}

#[test]
fn legacy_and_native_coverage_never_share_a_scope() {
    // Native first: every legacy registration in the scope is refused.
    let mut router = router(16, 4, 8);
    let native = router
        .register_native_world_shard(home(), generation())
        .unwrap();
    let conflict = SessionError::EndpointKindConflict {
        world: WorldId::new(0),
        dimension: DimensionId::new(0),
    };
    assert_eq!(
        router.register_world_shard(home()).err(),
        Some(conflict.clone())
    );
    assert_eq!(router.register_exact_shard(home()).err(), Some(conflict));
    // The legacy adapter fails closed with an already-closed receiver.
    let mut closed = router.register_shard(ShardPos::new(0, 0));
    assert!(closed.try_recv().is_err());
    assert_eq!(router.shard_count(), 1);
    // Legacy UUID-only joins and lease mutations never touch a native endpoint.
    assert_eq!(
        router
            .join_player(player("legacy"), "legacy", spawn())
            .err(),
        Some(SessionError::NativeEndpointRequiresSession { home: home() })
    );
    assert_eq!(router.player_count(), 0);
    assert_eq!(
        router.replace_shard_registration(&native.lease()).err(),
        Some(SessionError::NativeEndpointRequiresSession { home: home() })
    );
    assert_eq!(
        router.unregister_shard_registration(&native.lease()).err(),
        Some(SessionError::NativeEndpointRequiresSession { home: home() })
    );
    assert_eq!(router.shard_count(), 1);
    // A different dimension is a different scope.
    let other =
        ShardPartitioner::for_shard(WorldId::new(0), DimensionId::new(1), ShardPos::new(0, 0))
            .unwrap();
    assert!(router.register_world_shard(other).is_ok());

    // Legacy first: native registration in the scope is refused.
    let mut router = native_support::router(16, 4, 8);
    let legacy = router.register_world_shard(home()).unwrap();
    assert_eq!(
        router
            .register_native_world_shard(home(), generation())
            .err(),
        Some(CollectorSessionError::EndpointKindConflict {
            world: WorldId::new(0),
            dimension: DimensionId::new(0),
        })
    );
    // Native lease APIs refuse a legacy registration.
    let (_, _, receiver) = native.into_parts();
    assert_eq!(
        router
            .replace_native_shard_registration(&legacy.lease(), &receiver)
            .err(),
        Some(CollectorSessionError::LegacyEndpoint { home: home() })
    );
    let refused = router
        .unregister_native_shard_registration(&legacy.lease(), receiver)
        .unwrap_err();
    assert_eq!(
        refused.error(),
        &CollectorSessionError::LegacyEndpoint { home: home() }
    );
    assert_eq!(router.shard_count(), 1);
}

#[tokio::test]
async fn restore_and_bootstrap_admissions_queue_join_game_first_on_both_backends() {
    for store in STORES {
        let mut env = SimEnv::new(store).await;
        let mut router = router(64, 16, 8);
        let (_, _, lane) = router
            .register_native_world_shard(home(), generation())
            .unwrap()
            .into_parts();
        let restored = join(&mut router, &mut env, &lane, "Restored", spawn()).await;

        let position = Vec3::new(-0.0, 65.25, 2.125);
        let reservation = router
            .reserve_native_join(player("Fresh"), generation(), position)
            .unwrap();
        let handle = env.bootstrap(player("Fresh"), position).await;
        let mut fresh = admit(&mut router, &env, &lane, &reservation, &handle).unwrap();

        for (admitted, expected_wire) in [(&mut fresh, 3), (&mut join_session(restored), 2)] {
            let key = admitted.session_key().expect("native key");
            let status = router.native_session_status(&key).unwrap();
            assert_eq!(status.phase(), NativeSessionPhase::Active);
            assert_eq!(status.endpoint(), NativeEndpointState::Bound);
            assert!(status.ever_active());
            assert_eq!(status.self_wire_id(), 1);
            assert_eq!(status.remote_wire_id(), expected_wire);
            assert_eq!(status.disconnect(), None);
            let first = admitted.try_recv().expect("first outbound message");
            assert_eq!(first.criticality(), Criticality::Mandatory);
            assert_eq!(first.priority(), OutboundPriority::State);
            let ClientboundPlayPacket::JoinGame(join_game) = first.packet() else {
                panic!("the first outbound packet must be JoinGame");
            };
            assert_eq!(join_game.entity_id(), 1);
            assert_eq!(join_game.world_state().gamemode(), 0, "fresh Survival");
            assert_eq!(join_game.world_state().hashed_seed(), SEED_37_HASH);
            assert_wire_frame(
                join_game,
                JoinGame::encode,
                JoinGame::decode,
                JoinGame::PACKET_ID,
                None,
            )
            .expect("the queued JoinGame forms a valid decoder frame");
            // NET-04.2e2: after JoinGame exactly the other session's mandatory
            // tab-list add and spawn (e1 had no peer burst), nothing else.
            assert_eq!(drain_peer_burst(admitted), 2);
            assert!(admitted.try_recv().is_none());
        }

        // The projection carries the live view's exact bits, unnormalized.
        let projection = fresh.native_spawn_projection().unwrap();
        assert_eq!(projection.position().x.to_bits(), (-0.0_f64).to_bits());
        assert_eq!(projection.position().y.to_bits(), 65.25_f64.to_bits());
        assert_eq!(
            projection.look().map(f32::to_bits),
            [(-0.0_f32).to_bits(), 12.5_f32.to_bits()]
        );
        assert_eq!(projection.game_mode(), GameMode::Survival);
        assert_eq!(router.native_session_count(), 2);
        assert_eq!(router.native_reservation_count(), 0);
    }
}

fn join_session(admitted: Admitted) -> ferrumc_session::PlayerSessionHandle {
    admitted.session
}

#[tokio::test]
async fn every_join_game_field_mismatch_is_refused_before_any_side_effect() {
    type Mutate = fn(&mut JoinFields);
    let cases: [(Mutate, &str); 20] = [
        (|f| f.entity_id = 2, "self entity id"),
        (|f| f.hardcore = true, "hardcore"),
        (|f| f.world_names = vec![], "world names"),
        (
            |f| f.world_names = vec!["minecraft:overworld", "minecraft:the_nether"],
            "world names",
        ),
        (|f| f.max_players = 33, "max players"),
        (|f| f.view_distance = 11, "view distance"),
        (|f| f.simulation_distance = 9, "simulation distance"),
        (|f| f.reduced_debug = true, "reduced debug info"),
        (|f| f.respawn_screen = false, "respawn screen"),
        (|f| f.limited_crafting = true, "limited crafting"),
        (|f| f.secure_chat = true, "secure chat"),
        (|f| f.dimension_type = 1, "dimension type"),
        (|f| f.dimension_name = "minecraft:the_end", "dimension name"),
        (|f| f.hashed_seed = SEED_37_HASH ^ 1, "hashed seed"),
        (|f| f.gamemode = 1, "game mode"),
        (|f| f.previous_gamemode = 0, "previous game mode"),
        (|f| f.debug = true, "debug world"),
        (|f| f.flat = false, "flat world"),
        (|f| f.portal_cooldown = 1, "portal cooldown"),
        (|f| f.sea_level = 62, "sea level"),
    ];
    let mut env = SimEnv::new(Store::Memory).await;
    let mut router = router(64, 16, 8);
    let (_, _, lane) = router
        .register_native_world_shard(home(), generation())
        .unwrap()
        .into_parts();
    let reservation = router
        .reserve_native_join(player("Checked"), generation(), spawn())
        .unwrap();
    let handle = env.restore(player("Checked"), spawn()).await;
    let view = env.shard.connected_owned_player_view(&handle).unwrap();
    for (mutate, field) in cases {
        let mut fields = JoinFields::valid(GameMode::Survival);
        mutate(&mut fields);
        assert_eq!(
            router
                .admit_native_join(
                    &reservation,
                    &lane,
                    &handle,
                    &view,
                    fields.build(),
                    "c",
                    vec![]
                )
                .err(),
            Some(CollectorSessionError::JoinGameMismatch(field)),
            "{field}"
        );
        assert_eq!(router.native_session_count(), 0);
        assert_eq!(router.native_reservation_count(), 1);
    }
    let session = router.admit_native_join(
        &reservation,
        &lane,
        &handle,
        &view,
        JoinFields::valid(GameMode::Survival).build(),
        "c",
        vec![],
    );
    assert!(session.is_ok());
    assert_eq!(
        router
            .admit_native_join(
                &reservation,
                &lane,
                &handle,
                &view,
                JoinFields::valid(GameMode::Survival).build(),
                "c",
                vec![]
            )
            .err(),
        Some(CollectorSessionError::StaleReservation),
        "a join completes exactly once"
    );
}

#[tokio::test]
#[allow(
    clippy::too_many_lines,
    reason = "every admission refusal against one genuine pair of admissions"
)]
async fn admission_mismatches_are_refused_on_both_backends() {
    for store in STORES {
        let mut env = SimEnv::new(store).await;
        let mut router = router(64, 16, 8);
        let registration = router
            .register_native_world_shard(home(), generation())
            .unwrap();
        let lease = registration.lease();
        let (_, _, lane) = registration.into_parts();

        // Another router's reservation and receiver.
        let mut foreign = native_support::router(64, 16, 8);
        let (_, _, foreign_lane) = foreign
            .register_native_world_shard(home(), generation())
            .unwrap()
            .into_parts();
        let foreign_reservation = foreign
            .reserve_native_join(player("Alpha"), generation(), spawn())
            .unwrap();

        let alpha = router
            .reserve_native_join(player("Alpha"), generation(), spawn())
            .unwrap();
        let beta_position = Vec3::new(20.0, 64.0, 20.0);
        let beta = router
            .reserve_native_join(player("Beta"), generation(), beta_position)
            .unwrap();
        let alpha_handle = env.restore(player("Alpha"), spawn()).await;
        let beta_handle = env
            .restore(player("Beta"), Vec3::new(21.0, 64.0, 20.0))
            .await;

        let alpha_view = env
            .shard
            .connected_owned_player_view(&alpha_handle)
            .unwrap();
        let join = || JoinFields::valid(GameMode::Survival).build();
        assert_eq!(
            router
                .admit_native_join(
                    &foreign_reservation,
                    &lane,
                    &alpha_handle,
                    &alpha_view,
                    join(),
                    "a",
                    vec![]
                )
                .err(),
            Some(CollectorSessionError::ForeignReservation)
        );
        assert_eq!(
            router
                .admit_native_join(
                    &alpha,
                    &foreign_lane,
                    &alpha_handle,
                    &alpha_view,
                    join(),
                    "a",
                    vec![]
                )
                .err(),
            Some(CollectorSessionError::StaleReceiver)
        );
        assert_eq!(
            router
                .admit_native_join(
                    &beta,
                    &lane,
                    &alpha_handle,
                    &alpha_view,
                    join(),
                    "b",
                    vec![]
                )
                .err(),
            Some(CollectorSessionError::AdmissionMismatch("player"))
        );
        let beta_view = env.shard.connected_owned_player_view(&beta_handle).unwrap();
        assert_eq!(
            router
                .admit_native_join(
                    &alpha,
                    &lane,
                    &alpha_handle,
                    &beta_view,
                    join(),
                    "a",
                    vec![]
                )
                .err(),
            Some(CollectorSessionError::AdmissionMismatch("player"))
        );
        // Beta was declared at another position than the genuine admission.
        assert_eq!(
            router
                .admit_native_join(&beta, &lane, &beta_handle, &beta_view, join(), "b", vec![])
                .err(),
            Some(CollectorSessionError::AdmissionMismatch(
                "declared position"
            ))
        );
        assert_eq!(router.native_session_count(), 0);
        assert_eq!(router.native_reservation_count(), 2);

        // A rejoined admission's view never matches the earlier handle.
        env.detach_in_sim(&alpha_handle);
        let read = env
            .binding
            .read_player(&env.store, player("Alpha"))
            .await
            .unwrap();
        env.shard
            .enqueue_owned_player_input(ferrumc_sim::OwnedPlayerInput::rejoin(read))
            .unwrap();
        let out = env.boundary();
        let rejoined = admitted(&out);
        let rejoined_view = env.shard.connected_owned_player_view(&rejoined).unwrap();
        assert_eq!(
            router
                .admit_native_join(
                    &alpha,
                    &lane,
                    &alpha_handle,
                    &rejoined_view,
                    join(),
                    "a",
                    vec![]
                )
                .err(),
            Some(CollectorSessionError::AdmissionMismatch("handle"))
        );

        // A replaced registration's old receiver is stale for admission.
        let replaced = router
            .replace_native_shard_registration(&lease, &lane)
            .unwrap();
        assert_eq!(
            router
                .admit_native_join(
                    &alpha,
                    &lane,
                    &rejoined,
                    &rejoined_view,
                    join(),
                    "a",
                    vec![]
                )
                .err(),
            Some(CollectorSessionError::StaleReceiver)
        );
        let (_, _, current) = replaced.into_parts();
        assert!(router
            .admit_native_join(
                &alpha,
                &current,
                &rejoined,
                &rejoined_view,
                join(),
                "a",
                vec![]
            )
            .is_ok());
    }
}

#[tokio::test]
#[allow(
    clippy::too_many_lines,
    reason = "the complete reservation refusal table and N-1/N/N+1"
)]
async fn reservation_refusals_precede_every_side_effect() {
    let mut router = router(64, 16, 2);
    assert_eq!(
        router
            .reserve_native_join(player("NoEndpoint"), generation(), spawn())
            .err(),
        Some(CollectorSessionError::UnknownShard {
            target: ShardPartitioner::for_shard(
                WorldId::new(0),
                DimensionId::new(0),
                ShardPos::new(0, 0)
            )
            .unwrap()
        })
    );
    let registration = router
        .register_native_world_shard(home(), generation())
        .unwrap();
    let unsupported = GenerationBinding::new(
        GenerationKey::new(WorldId::new(3), DimensionId::new(0)),
        identity(),
    );
    let seeded = GenerationBinding::new(
        scope(),
        GenerationIdentity::flat_overworld(WorldSeed::new(2)),
    );
    let cases = [
        (
            unsupported,
            spawn(),
            CollectorSessionError::Unsupported("declared scope"),
        ),
        (
            seeded,
            spawn(),
            CollectorSessionError::AdmissionMismatch("declared scope"),
        ),
        (
            generation(),
            Vec3::new(128.0, 64.0, 8.0),
            CollectorSessionError::NativeAdmissionOutsideRegion,
        ),
        (
            generation(),
            Vec3::new(8.0, 64.0, -0.000_001),
            CollectorSessionError::NativeAdmissionOutsideRegion,
        ),
        (
            generation(),
            Vec3::new(f64::NAN, 64.0, 8.0),
            CollectorSessionError::InvalidTransform,
        ),
        (
            generation(),
            Vec3::new(8.0, f64::INFINITY, 8.0),
            CollectorSessionError::InvalidTransform,
        ),
        (
            generation(),
            Vec3::new(3.1e7, 64.0, 8.0),
            CollectorSessionError::InvalidTransform,
        ),
    ];
    for (scope, position, expected) in cases {
        assert_eq!(
            router
                .reserve_native_join(player("Refused"), scope, position)
                .err(),
            Some(expected)
        );
    }
    assert_eq!(router.native_reservation_count(), 0);

    // N-1, N, N+1 against the session limit 2; cancel frees the counted entry.
    let first = router
        .reserve_native_join(player("One"), generation(), spawn())
        .unwrap();
    assert_eq!(
        router
            .reserve_native_join(player("One"), generation(), spawn())
            .err(),
        Some(CollectorSessionError::DuplicatePlayer {
            player: player("One")
        })
    );
    let second = router
        .reserve_native_join(player("Two"), generation(), spawn())
        .unwrap();
    assert_eq!(
        router
            .reserve_native_join(player("Three"), generation(), spawn())
            .err(),
        Some(CollectorSessionError::SessionCapacity { limit: 2 })
    );
    assert_eq!(router.native_reservation_count(), 2);
    assert_eq!(router.cancel_native_join(first), Ok(player("One")));
    let third = router
        .reserve_native_join(player("Three"), generation(), spawn())
        .unwrap();
    assert_eq!(third.player(), player("Three"));

    // Unregister refuses while reservations reference the registration and
    // returns the receiver intact.
    let lease = registration.lease();
    let (_, _, receiver) = registration.into_parts();
    let refused = router
        .unregister_native_shard_registration(&lease, receiver)
        .unwrap_err();
    assert_eq!(
        refused.error(),
        &CollectorSessionError::NativeEndpointInUse {
            registration_id: lease.registration_id(),
            sessions: 0,
            reservations: 2,
        }
    );
    let (_, receiver) = refused.into_parts();
    assert_eq!(router.cancel_native_join(second), Ok(player("Two")));
    assert_eq!(router.cancel_native_join(third), Ok(player("Three")));
    assert_eq!(router.native_reservation_count(), 0);
    assert_eq!(
        router
            .unregister_native_shard_registration(&lease, receiver)
            .map_err(|e| e.into_parts().0),
        Ok(home())
    );
    assert_eq!(router.shard_count(), 0);
}

#[tokio::test]
async fn cancelled_reservations_never_reissue_wire_ids_or_incarnations() {
    let mut env = SimEnv::new(Store::Memory).await;
    let mut router = router(64, 16, 8);
    let (_, _, lane) = router
        .register_native_world_shard(home(), generation())
        .unwrap()
        .into_parts();
    let cancelled = router
        .reserve_native_join(player("Again"), generation(), spawn())
        .unwrap();
    let foreign_cancel = native_support::router(64, 16, 8)
        .cancel_native_join(
            router
                .reserve_native_join(player("Other"), generation(), spawn())
                .unwrap(),
        )
        .err();
    assert_eq!(
        foreign_cancel,
        Some(CollectorSessionError::ForeignReservation)
    );
    router.cancel_native_join(cancelled).unwrap();
    let admitted = join(&mut router, &mut env, &lane, "Again", spawn()).await;
    let status = router
        .native_session_status(&admitted.session.session_key().unwrap())
        .unwrap();
    // Wire ids 2 and 3 were burned by the two earlier reservations.
    assert_eq!(status.remote_wire_id(), 4);
}

#[tokio::test]
async fn legacy_joins_count_against_the_initialized_session_limit() {
    let mut router = router(64, 16, 2);
    let _registration = router.register_exact_shard(home()).unwrap();
    let _one = router.join_player(player("L1"), "L1", spawn()).unwrap();
    let _two = router.join_player(player("L2"), "L2", spawn()).unwrap();
    assert_eq!(
        router.join_player(player("L3"), "L3", spawn()).err(),
        Some(SessionError::SessionCapacity { limit: 2 })
    );
    assert_eq!(router.player_count(), 2);
    // A legacy session blocks a native reservation of the same UUID.
    assert!(matches!(
        router.reserve_native_join(player("L1"), generation(), spawn()),
        Err(CollectorSessionError::LegacyEndpoint { .. }
            | CollectorSessionError::DuplicatePlayer { .. })
    ));
}
