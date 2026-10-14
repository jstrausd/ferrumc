use super::*;
use crate::{GroundItemDrop, GroundItemError};
use ferrumc_items::ComponentPatch;
use std::num::NonZeroU8;

fn death_drop(position: Vec3, name: &str, count: u8) -> GroundItemDrop {
    GroundItemDrop::new(position, stack(name, count), 40, None, None)
}
async fn death_owner(p: PlayerId, cap: usize) -> SimShard {
    let mut s = shard_with_player(p).await;
    enable_ground_runtime(&mut s, cap);
    s.enable_survival(crate::Difficulty::Normal, false)
        .expect("vitals");
    s
}
fn die(s: &mut SimShard, p: PlayerId) {
    s.enqueue(GameInput::PlayerDamage {
        player: p,
        request: crate::DamageRequest::new(100.0, 0.0, 0.0).expect("lethal"),
    })
    .expect("damage");
    assert!(s
        .run_tick()
        .iter()
        .any(|output| matches!(output,GameOutput::PlayerDied {player} if *player==p)));
}

#[tokio::test]
async fn death_batch_owner_alive_and_wrong_position_refuse_every_original_stack() {
    let p = player("death_guard");
    let mut s = death_owner(p, 4).await;
    let position = s.player_position(p).expect("actual pose");
    let drops = vec![
        death_drop(position, "diamond", 3),
        death_drop(position, "oak_log", 7),
    ];
    let refusal = s.enqueue_player_death_drops(p, drops).expect_err("alive");
    assert!(matches!(refusal.reason(), GroundItemError::InvalidDeath));
    let (_, drops) = refusal.into_parts();
    assert_eq!(drops.len(), 2);
    assert_eq!(drops[0].stack, stack("diamond", 3));
    assert!(s.ground_items.as_ref().expect("owner").pending.is_empty());
    die(&mut s, p);
    let wrong = position + Vec3::new(0.0, 1.0, 0.0);
    let refusal = s
        .enqueue_player_death_drops(
            p,
            vec![
                death_drop(position, "diamond", 3),
                death_drop(wrong, "oak_log", 7),
            ],
        )
        .expect_err("position");
    assert_eq!(refusal.into_parts().1.len(), 2);
    assert_eq!(s.player_death_drops_admitted(p), Some(false));
    assert!(s.ground_items.as_ref().expect("owner").pending.is_empty());
}

#[tokio::test]
async fn death_batch_owner_complete_capacity_preflight_then_once_per_life_and_respawn() {
    let p = player("death_once");
    let mut s = death_owner(p, 4).await;
    let pos = s.player_position(p).expect("death position");
    die(&mut s, p);
    let original = s.ground_items.as_ref().expect("owner").item_bytes;
    let refusal = s
        .enqueue_player_death_drops(p, (0..5).map(|_| death_drop(pos, "diamond", 3)).collect())
        .expect_err("whole capacity");
    assert_eq!(refusal.into_parts().1.len(), 5);
    assert_eq!(s.player_death_drops_admitted(p), Some(false));
    assert_eq!(s.ground_items.as_ref().expect("owner").item_bytes, original);
    assert!(s.ground_items.as_ref().expect("owner").pending.is_empty());
    s.enqueue_player_death_drops(
        p,
        vec![death_drop(pos, "diamond", 3), death_drop(pos, "oak_log", 7)],
    )
    .expect("whole batch");
    assert_eq!(s.player_death_drops_admitted(p), Some(true));
    assert_eq!(s.ground_items.as_ref().expect("owner").pending.len(), 2);
    let refusal = s
        .enqueue_player_death_drops(p, vec![death_drop(pos, "diamond", 3)])
        .expect_err("replayed death");
    assert!(matches!(
        refusal.reason(),
        GroundItemError::DeathAlreadyAdmitted
    ));
    assert_eq!(refusal.into_parts().1[0].stack, stack("diamond", 3));
    s.enqueue(GameInput::Respawn {
        player: p,
        position: pos,
    })
    .expect("respawn");
    let _ = s.run_tick();
    assert_eq!(s.player_death_drops_admitted(p), Some(false));
    die(&mut s, p);
    s.enqueue_player_death_drops(p, Vec::new())
        .expect("new empty life");
    assert_eq!(s.player_death_drops_admitted(p), Some(true));
    assert_eq!(s.ground_items.as_ref().expect("owner").pending.len(), 2);
}

#[tokio::test]
async fn death_batch_owner_invalid_late_metadata_and_max56_do_not_partially_admit() {
    let p = player("death_metadata");
    let mut s = death_owner(p, 64).await;
    let pos = s.player_position(p).expect("position");
    die(&mut s, p);
    let present_air = ItemStack::new(
        ferrumc_items::ItemId::from_name("air").expect("air"),
        NonZeroU8::MIN,
        ComponentPatch::empty(),
    );
    let refusal = s
        .enqueue_player_death_drops(
            p,
            vec![
                death_drop(pos, "diamond", 3),
                GroundItemDrop::new(pos, present_air.clone(), 40, None, None),
            ],
        )
        .expect_err("late invalid metadata");
    let (_, retained) = refusal.into_parts();
    assert_eq!(retained[0].stack, stack("diamond", 3));
    assert_eq!(retained[1].stack, present_air);
    assert!(s.ground_items.as_ref().expect("owner").pending.is_empty());
    assert!(s
        .enqueue_player_death_drops(p, (0..57).map(|_| death_drop(pos, "diamond", 1)).collect())
        .is_err());
    assert!(s.ground_items.as_ref().expect("owner").pending.is_empty());
    s.enqueue_player_death_drops(p, (0..56).map(|_| death_drop(pos, "diamond", 1)).collect())
        .expect("complete bound56");
    assert_eq!(s.ground_items.as_ref().expect("owner").pending.len(), 56);
    assert!(
        s.capture_ground_item_records().is_err(),
        "pending moved payload is not a durable save receipt"
    );
}

#[tokio::test]
async fn death_batch_owner_generic_multi_region_claims_preflight_whole_batch() {
    let p = player("batch_regions");
    let mut s = death_owner(p, 128).await;
    for from in [0, 32] {
        let drops = (from..from + 32)
            .map(|x| {
                death_drop(
                    Vec3::new(f64::from(x) * 128.0 + 8.0, 64.0, 8.0),
                    "diamond",
                    1,
                )
            })
            .collect();
        s.enqueue_ground_item_drops(drops)
            .expect("64distinct canonical claims");
    }
    let before = s.ground_items.as_ref().expect("owner").item_bytes;
    let pos = Vec3::new(8.0, 64.0, 8.0);
    let other = Vec3::new(64.0 * 128.0 + 8.0, 64.0, 8.0);
    let refusal = s
        .enqueue_ground_item_drops(vec![
            death_drop(pos, "diamond", 7),
            death_drop(other, "oak_log", 4),
        ])
        .expect_err("whole claims refusal");
    let (_, retained) = refusal.into_parts();
    assert_eq!(retained.len(), 2);
    assert_eq!(retained[0].stack, stack("diamond", 7));
    assert_eq!(retained[1].stack, stack("oak_log", 4));
    let ground = s.ground_items.as_ref().expect("owner");
    assert_eq!(ground.pending.len(), 64);
    assert_eq!(ground.item_bytes, before);
    assert_eq!(ground.region_claims.len(), 64);
}

#[tokio::test]
async fn death_batch_owner_actual_full_publication_refuses_before_any_death_admission() {
    let p = player("death_pressure");
    let mut s = death_owner(p, 8).await;
    s.refill_ground_item_identities((90..94).map(ground_identity).collect())
        .expect("four identities");
    for x in 4..8 {
        s.enqueue_ground_item_drop(GroundItemDrop::new(
            Vec3::new(f64::from(x), 300.0, 8.0),
            stack("diamond", 1),
            32767,
            None,
            None,
        ))
        .expect("real falling item");
    }
    for _ in 0..200 {
        let _ = s.run_tick();
    }
    assert_eq!(
        s.ground_items.as_ref().expect("owner").events.len(),
        crate::ground_items::MAX_GROUND_EVENTS - 1,
        "actual fall movement exhausted space for a complete two-drop batch"
    );
    die(&mut s, p);
    let pos = s.player_position(p).expect("death pose");
    let refusal = s
        .enqueue_player_death_drops(
            p,
            vec![death_drop(pos, "oak_log", 7), death_drop(pos, "diamond", 2)],
        )
        .expect_err("full publication");
    assert!(matches!(
        refusal.reason(),
        GroundItemError::Capacity("batch publication room")
    ));
    let retained = refusal.into_parts().1;
    assert_eq!(retained[0].stack, stack("oak_log", 7));
    assert_eq!(retained[1].stack, stack("diamond", 2));
    assert_eq!(s.player_death_drops_admitted(p), Some(false));
    assert!(s.ground_items.as_ref().expect("owner").pending.is_empty());
    let _ = s.take_ground_item_events(256).expect("real observer drain");
    s.enqueue_player_death_drops(
        p,
        vec![death_drop(pos, "oak_log", 7), death_drop(pos, "diamond", 2)],
    )
    .expect("after output room");
    assert_eq!(s.player_death_drops_admitted(p), Some(true));
    assert_eq!(s.ground_items.as_ref().expect("owner").pending.len(), 2);
}
