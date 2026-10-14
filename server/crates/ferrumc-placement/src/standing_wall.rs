//! Normal top/side attachment projection from pinned Paper 1.21.8 Build60.
//! Either default alias may arrive from the registry's reverse item mapping.
//! Full nearest-looking-direction support search and skull neighbor signal are
//! absent from `PlacementContext`; downward clicks keep the previous fallback.

use super::{encode, facing_string, is_horizontal, PlacementContext, PlacementRule};
use std::collections::BTreeMap;

const BANNERS: [(&str, &str); 16] = [
    ("white_banner", "white_wall_banner"),
    ("orange_banner", "orange_wall_banner"),
    ("magenta_banner", "magenta_wall_banner"),
    ("light_blue_banner", "light_blue_wall_banner"),
    ("yellow_banner", "yellow_wall_banner"),
    ("lime_banner", "lime_wall_banner"),
    ("pink_banner", "pink_wall_banner"),
    ("gray_banner", "gray_wall_banner"),
    ("light_gray_banner", "light_gray_wall_banner"),
    ("cyan_banner", "cyan_wall_banner"),
    ("purple_banner", "purple_wall_banner"),
    ("blue_banner", "blue_wall_banner"),
    ("brown_banner", "brown_wall_banner"),
    ("green_banner", "green_wall_banner"),
    ("red_banner", "red_wall_banner"),
    ("black_banner", "black_wall_banner"),
];
const SKULLS: [(&str, &str); 7] = [
    ("skeleton_skull", "skeleton_wall_skull"),
    ("wither_skeleton_skull", "wither_skeleton_wall_skull"),
    ("player_head", "player_wall_head"),
    ("zombie_head", "zombie_wall_head"),
    ("creeper_head", "creeper_wall_head"),
    ("dragon_head", "dragon_wall_head"),
    ("piglin_head", "piglin_wall_head"),
];

pub(super) fn variants(name: &str) -> Option<(&'static str, &'static str, bool)> {
    BANNERS
        .iter()
        .find(|(floor, wall)| name == *floor || name == *wall)
        .map(|&(floor, wall)| (floor, wall, true))
        .or_else(|| {
            SKULLS
                .iter()
                .find(|(floor, wall)| name == *floor || name == *wall)
                .map(|&(floor, wall)| (floor, wall, false))
        })
}

pub(super) fn place(name: &str, fallback: u32, ctx: &PlacementContext) -> (u32, PlacementRule) {
    let Some((floor, wall, banner)) = variants(name) else {
        return (fallback, PlacementRule::SimpleCube);
    };
    let mut props = BTreeMap::new();
    let target = if is_horizontal(ctx.clicked_face) {
        props.insert("facing", facing_string(ctx.clicked_face));
        wall
    } else if ctx.clicked_face == ferrumc_math::Direction::Up && ctx.player_yaw.is_finite() {
        let angle = if banner {
            ctx.player_yaw + 180.0_f32
        } else {
            ctx.player_yaw
        };
        // SegmentedAnglePrecision uses float multiplication, Java Math.round
        // (negative halves round toward positive infinity), then a 16-way mask.
        // These are angle segments, never registry state-id multipliers.
        let segment = (f64::from(angle * (16.0_f32 / 360.0_f32)) + 0.5).floor() as i32 & 15;
        let rotations = [
            "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15",
        ];
        props.insert("rotation", rotations[segment as usize]);
        floor
    } else {
        // The context cannot prove an alternate supporting wall after a ceiling
        // click; keep existing fallback behavior instead of inventing support.
        return (fallback, PlacementRule::SimpleCube);
    };
    (
        encode(target, &props, fallback),
        PlacementRule::StandingAndWall,
    )
}

#[cfg(test)]
mod tests;
