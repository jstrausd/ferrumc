//! Placement collision geometry from the pinned original Paper runtime.

use ferrumc_math::{Aabb, BlockPos, Vec3};

#[path = "collision_data.rs"]
mod data;

const STATE_SHAPES: &[u8] = include_bytes!("../data/collision/state-shapes.bin");
// Shapes.joinIsNotEmpty merges axis coordinates closer than this epsilon.
const COLLISION_EPSILON: f64 = 1.0e-7;

/// Returns the original standing-player body at authoritative feet coordinates.
///
/// Width and height retain Minecraft's `float` representation before promotion
/// to doubles: `EntityDimensions.scalable(0.6F, 1.8F).makeBoundingBox`. Nonfinite
/// coordinates refuse. Other poses require their own authoritative dimensions;
/// this helper does not infer crouching/swimming from an untrusted client.
#[must_use]
pub fn standing_player_body(feet: Vec3) -> Option<Aabb> {
    let half_width = f64::from(0.6_f32 / 2.0);
    let height = f64::from(1.8_f32);
    let min = Vec3::new(feet.x - half_width, feet.y, feet.z - half_width);
    let max = Vec3::new(feet.x + half_width, feet.y + height, feet.z + half_width);
    if [min.x, min.y, min.z, max.x, max.y, max.z]
        .iter()
        .any(|coordinate| !coordinate.is_finite())
    {
        return None;
    }
    Some(Aabb::new(min, max))
}

/// Tests a final placed state against one authoritative entity body.
///
/// Uses the state's genuine placement-context boxes, translated to `position`.
/// Empty collision shapes return `Some(false)`; touching faces/edges are clear.
/// `None` means an invalid state/body or unsupported contextual shape, so the
/// authoritative caller must refuse rather than guess a full or empty cube.
/// Colliding position-offset bamboo/dripstone and moving-piston shapes currently
/// refuse; fresh closed shulkers and placement-empty scaffolding/powder snow are
/// supported. This is a placement check, not movement/support-shape physics.
#[must_use]
pub fn placement_intersects_body(state_id: u32, position: BlockPos, body: Aabb) -> Option<bool> {
    let min = body.min();
    let max = body.max();
    if [min.x, min.y, min.z, max.x, max.y, max.z]
        .iter()
        .any(|coordinate| !coordinate.is_finite())
        || min.x >= max.x
        || min.y >= max.y
        || min.z >= max.z
    {
        return None;
    }
    let offset = usize::try_from(state_id).ok()?.checked_mul(2)?;
    let bytes = STATE_SHAPES.get(offset..offset.checked_add(2)?)?;
    let shape = data::SHAPES.get(usize::from(u16::from_le_bytes([bytes[0], bytes[1]])))?;
    let origin = [
        f64::from(position.x()),
        f64::from(position.y()),
        f64::from(position.z()),
    ];
    let min = [min.x, min.y, min.z];
    let max = [max.x, max.y, max.z];
    Some(shape.iter().any(|bounds| {
        (0..3).all(|axis| {
            (origin[axis] + bounds[axis + 3]).min(max[axis])
                - (origin[axis] + bounds[axis]).max(min[axis])
                > COLLISION_EPSILON
        })
    }))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use ferrumc_registry::block_state::{block_default_state, compute_state_id};

    use super::*;

    fn intersects(name: &str, feet: Vec3) -> Option<bool> {
        placement_intersects_body(
            block_default_state(name).unwrap(),
            BlockPos::new(0, 0, 0),
            standing_player_body(feet).unwrap(),
        )
    }

    #[test]
    fn standing_dimensions_retain_original_float_promotion() {
        let body = standing_player_body(Vec3::new(8.0, 64.0, 8.0)).unwrap();
        assert_eq!(body.min().x, 8.0 - f64::from(0.6_f32 / 2.0));
        assert_eq!(body.max().y, 64.0 + f64::from(1.8_f32));
        assert!(standing_player_body(Vec3::new(f64::NAN, 0.0, 0.0)).is_none());
    }

    #[test]
    fn cube_overlap_boundary_and_noncolliding_blocks_differ() {
        assert_eq!(intersects("stone", Vec3::new(0.5, 0.0, 0.5)), Some(true));
        assert_eq!(
            intersects("stone", Vec3::new(1.0 + f64::from(0.6_f32 / 2.0), 0.0, 0.5)),
            Some(false)
        );
        for name in [
            "torch",
            "wall_torch",
            "short_grass",
            "scaffolding",
            "powder_snow",
        ] {
            assert_eq!(
                intersects(name, Vec3::new(0.5, 0.0, 0.5)),
                Some(false),
                "{name}"
            );
        }
        for name in ["bamboo", "pointed_dripstone", "moving_piston"] {
            assert_eq!(intersects(name, Vec3::new(0.5, 0.0, 0.5)), None, "{name}");
        }
        assert_eq!(
            placement_intersects_body(
                u32::MAX,
                BlockPos::new(0, 0, 0),
                standing_player_body(Vec3::ZERO).unwrap()
            ),
            None
        );
    }

    #[test]
    fn slab_shapes_preserve_the_clear_half() {
        let body = standing_player_body(Vec3::new(0.5, 0.5, 0.5)).unwrap();
        for (kind, expected) in [("bottom", false), ("top", true), ("double", true)] {
            let state = compute_state_id("oak_slab", &BTreeMap::from([("type", kind)])).unwrap();
            assert_eq!(
                placement_intersects_body(state, BlockPos::new(0, 0, 0), body),
                Some(expected),
                "{kind}"
            );
        }
    }

    #[test]
    fn complete_supported_states_match_original_geometric_probes() {
        #[derive(serde::Deserialize)]
        struct Probe {
            position: [i32; 3],
            feet: [f64; 3],
        }
        let feet: Vec<Probe> =
            serde_json::from_str(include_str!("../data/collision/probe-feet.json")).unwrap();
        let golden = include_bytes!("../data/collision/probe-results.bin");
        assert_eq!(golden.len(), STATE_SHAPES.len() / 2 * feet.len());
        let mut checked = 0;
        for (state, rows) in golden.chunks_exact(feet.len()).enumerate() {
            for (case, probe) in feet.iter().enumerate() {
                let feet = probe.feet;
                let body = standing_player_body(Vec3::new(feet[0], feet[1], feet[2])).unwrap();
                if let Some(actual) = placement_intersects_body(
                    u32::try_from(state).unwrap(),
                    BlockPos::new(probe.position[0], probe.position[1], probe.position[2]),
                    body,
                ) {
                    assert_eq!(
                        actual,
                        rows[case] == 1,
                        "original state {state}, body case {case}"
                    );
                    checked += 1;
                }
            }
        }
        assert_eq!(
            checked, 446_432,
            "complete supported original probe coverage"
        );
    }
}
