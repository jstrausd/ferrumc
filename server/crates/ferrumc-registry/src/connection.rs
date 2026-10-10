//! Pinned block-family facts used by fence connection rules.
//!
//! These allocation-free lookups describe the built-in Minecraft 1.21.8 tags
//! and block classes reviewed in Paper Build 60, commit
//! `29c8822d90899c89d2689338e81a98f690bcba12`. They do not load datapack tag
//! overrides or evaluate collision/support shapes. A known non-exception is
//! therefore not proof that its face can support a fence.

use crate::block_state::block_metadata;

/// Built-in fence families that connect to their own family.
///
/// Minecraft's fence rule compares `WOODEN_FENCES` tag membership, rather than
/// the dimension where a fence's material originates. Both nether wood types
/// therefore belong to [`Self::Wooden`]. Fence gates are a separate rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FenceFamily {
    /// The twelve fences in the pinned `WOODEN_FENCES` tag, including bamboo,
    /// crimson and warped fences.
    Wooden,
    /// The only pinned `FENCES` member outside `WOODEN_FENCES`: nether brick.
    NetherBrick,
}

const WOODEN_FENCES: [&str; 12] = [
    "oak_fence",
    "acacia_fence",
    "dark_oak_fence",
    "pale_oak_fence",
    "spruce_fence",
    "birch_fence",
    "jungle_fence",
    "crimson_fence",
    "warped_fence",
    "mangrove_fence",
    "bamboo_fence",
    "cherry_fence",
];

// In this pin the built-in LEAVES tag and all registered LeavesBlock-derived
// classes have the same eleven members. The exception's source uses the class,
// so future datapack tag overrides cannot stand in for that class relationship.
const LEAF_BLOCKS: [&str; 11] = [
    "jungle_leaves",
    "oak_leaves",
    "spruce_leaves",
    "pale_oak_leaves",
    "dark_oak_leaves",
    "acacia_leaves",
    "birch_leaves",
    "azalea_leaves",
    "flowering_azalea_leaves",
    "mangrove_leaves",
    "cherry_leaves",
];

const SHULKER_BOXES: [&str; 17] = [
    "shulker_box",
    "black_shulker_box",
    "blue_shulker_box",
    "brown_shulker_box",
    "cyan_shulker_box",
    "gray_shulker_box",
    "green_shulker_box",
    "light_blue_shulker_box",
    "light_gray_shulker_box",
    "lime_shulker_box",
    "magenta_shulker_box",
    "orange_shulker_box",
    "pink_shulker_box",
    "purple_shulker_box",
    "red_shulker_box",
    "white_shulker_box",
    "yellow_shulker_box",
];

const OTHER_CONNECTION_EXCEPTIONS: [&str; 5] = [
    "barrier",
    "carved_pumpkin",
    "jack_o_lantern",
    "melon",
    "pumpkin",
];

/// Returns the built-in fence family for a pinned registry block name.
///
/// Accepts bare or canonical `minecraft:` names. Returns `None` for a known
/// non-fence, an unknown name, or another namespace. Membership comes from the
/// pinned tags, not a suffix or the block's bounding box.
#[must_use]
pub fn fence_family(name: &str) -> Option<FenceFamily> {
    let name = block_metadata(name)?.name;
    if WOODEN_FENCES.contains(&name) {
        Some(FenceFamily::Wooden)
    } else if name == "nether_brick_fence" {
        Some(FenceFamily::NetherBrick)
    } else {
        None
    }
}

/// Classifies a known block as a built-in connection exception.
///
/// `Some(true)` identifies leaves, barrier, pumpkins, melon or shulker boxes
/// excluded by the reviewed `Block.isExceptionForConnection` rule.
/// `Some(false)` means the known block is not an exception; it does **not** imply
/// a full supporting face or a connectable fence/gate. Unknown or malformed
/// names and other namespaces return `None`, rather than a known false result.
/// Accepts bare or canonical `minecraft:` names and applies to all states of
/// the block under the built-in tags; runtime datapack overrides are not read.
#[must_use]
pub fn is_connection_exception(name: &str) -> Option<bool> {
    let name = block_metadata(name)?.name;
    Some(
        LEAF_BLOCKS.contains(&name)
            || SHULKER_BOXES.contains(&name)
            || OTHER_CONNECTION_EXCEPTIONS.contains(&name),
    )
}

#[cfg(test)]
mod tests;
