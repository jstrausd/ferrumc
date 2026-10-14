//! Ordinary unenchanted tool mining and deterministic block-drop contracts.

use ferrumc_items::{ComponentPatch, ItemId, ItemStack};
use ferrumc_registry::{block_state, item};

/// Authoritative held-item and environment facts supplied by the application.
///
/// This describes ordinary registry tools. Modified tool components,
/// enchantments and status effects require a separate effective-component
/// contract; they are not inferred from a protocol item id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MiningTool {
    held_item: Option<i32>,
    on_ground: bool,
    submerged: bool,
    token: u64,
}

impl MiningTool {
    /// Constructs a context, rejecting unknown protocol item ids.
    pub fn new(held_item: Option<i32>, on_ground: bool, submerged: bool) -> Option<Self> {
        if held_item.is_some_and(|id| item::lookup_item_name(id).is_none()) {
            return None;
        }
        Some(Self {
            held_item,
            on_ground,
            submerged,
            token: 0,
        })
    }

    /// Returns a grounded bare-hand context.
    pub const fn empty() -> Self {
        Self {
            held_item: None,
            on_ground: true,
            submerged: false,
            token: 0,
        }
    }

    /// Returns the exact registry item id, without granting inventory authority.
    pub const fn held_item(self) -> Option<i32> {
        self.held_item
    }

    /// Refreshes environment facts without changing the exact held-stack witness.
    #[must_use]
    pub const fn with_environment(mut self, on_ground: bool, submerged: bool) -> Self {
        self.on_ground = on_ground;
        self.submerged = submerged;
        self
    }

    pub(crate) const fn on_ground(self) -> bool {
        self.on_ground
    }

    /// Associates an immutable caller witness without transferring inventory authority.
    #[must_use]
    pub const fn with_token(mut self, token: u64) -> Self {
        self.token = token;
        self
    }

    /// Returns the caller witness for the exact committed held-stack snapshot.
    pub const fn token(self) -> u64 {
        self.token
    }

    fn family_and_tier(self) -> (u8, Option<usize>, f32) {
        let name = self
            .held_item
            .and_then(item::lookup_item_name)
            .unwrap_or("")
            .trim_start_matches("minecraft:");
        let family = if name.ends_with("_pickaxe") {
            1
        } else if name.ends_with("_axe") {
            2
        } else if name.ends_with("_shovel") {
            4
        } else if name.ends_with("_hoe") {
            8
        } else if name.ends_with("_sword") {
            16
        } else if name == "shears" {
            32
        } else {
            0
        };
        let (tier, speed) = if name.starts_with("wooden_") {
            (Some(0), 2.0)
        } else if name.starts_with("stone_") {
            (Some(1), 4.0)
        } else if name.starts_with("golden_") {
            (Some(2), 12.0)
        } else if name.starts_with("iron_") {
            (Some(3), 6.0)
        } else if name.starts_with("diamond_") {
            (Some(4), 8.0)
        } else if name.starts_with("netherite_") {
            (Some(5), 9.0)
        } else {
            (None, 1.0)
        };
        (family, tier, speed)
    }

    pub(crate) fn profile(self, state: crate::BlockStateId) -> Option<MiningProfile> {
        let name = block_state::state_id_to_block_name(state.as_u32())?;
        if matches!(name, "air" | "cave_air" | "void_air" | "water" | "lava") {
            return None;
        }
        let block = block_state::block_metadata(name)?;
        let &(hardness, families, harvest) =
            crate::mining_facts::BLOCK_MINING.get(usize::from(block.id))?;
        if hardness < 0.0 {
            return None;
        }
        let (family, tier, speed) = self.family_and_tier();
        let correct = if name == "cobweb" {
            matches!(family, 16 | 32)
        } else {
            harvest == 0 || family == 1 && tier.is_some_and(|tier| harvest & (1 << tier) != 0)
        };
        let mut speed = mining_speed(name, family, families, speed);
        if self.submerged {
            speed /= 5.0;
        }
        if !self.on_ground {
            speed /= 5.0;
        }
        // Original positive-speed / zero-hardness progress is infinite. Applying
        // an artificial 1.0 cap before environment penalties delayed grass mining.
        let progress = speed / hardness / if correct { 30.0 } else { 100.0 };
        Some(MiningProfile {
            progress,
            correct,
            tool_damage: if family == 32 && !matches!(name, "fire" | "soul_fire") {
                1
            } else if hardness == 0.0 || family == 0 {
                0
            } else if family == 16 {
                2
            } else {
                1
            },
        })
    }
}

fn mining_speed(name: &str, family: u8, families: u8, material_speed: f32) -> f32 {
    // Ordered special rules come from the complete 1.21.8 Tool components,
    // sword tags and ShearsItem, rather than material suffixes alone.
    match family {
        16 if name == "cobweb" => 15.0,
        16 if matches!(name, "bamboo" | "bamboo_sapling") => f32::MAX,
        16 if name.ends_with("_leaves")
            || matches!(
                name,
                "vine"
                    | "glow_lichen"
                    | "pumpkin"
                    | "carved_pumpkin"
                    | "jack_o_lantern"
                    | "melon"
                    | "cocoa"
                    | "big_dripleaf"
                    | "big_dripleaf_stem"
                    | "chorus_plant"
                    | "chorus_flower"
            ) =>
        {
            1.5
        }
        32 if name == "cobweb" || name.ends_with("_leaves") => 15.0,
        32 if name.ends_with("_wool") => 5.0,
        32 if matches!(name, "vine" | "glow_lichen") => 2.0,
        _ if family & families != 0 => material_speed,
        _ => 1.0,
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct MiningProfile {
    pub(crate) progress: f32,
    pub(crate) correct: bool,
    pub(crate) tool_damage: u8,
}

pub(crate) fn ordinary_drop(state: crate::BlockStateId, random: &mut u64) -> Option<ItemStack> {
    mined_drop(state, MiningTool::empty(), false, random)
}

/// Plans one ordinary unenchanted drop, using the intact pair before either edit.
#[allow(clippy::too_many_lines)] // One bounded original loot dispatch keeps each alternative explicit.
pub(crate) fn mined_drop(
    state: crate::BlockStateId,
    tool: MiningTool,
    intact_double_plant: bool,
    random: &mut u64,
) -> Option<ItemStack> {
    let name = block_state::state_id_to_block_name(state.as_u32())?;
    let shears = tool.family_and_tier().0 == 32;
    let (drop, count) = match name {
        "short_grass" | "fern" => {
            if shears {
                (name, 1)
            } else if sample(random, 0, 7) == 0 {
                ("wheat_seeds", 1)
            } else {
                return None;
            }
        }
        "tall_grass" | "large_fern" => {
            if !intact_double_plant {
                return None;
            }
            if shears {
                (
                    if name == "tall_grass" {
                        "short_grass"
                    } else {
                        "fern"
                    },
                    2,
                )
            } else if sample(random, 0, 7) == 0 {
                ("wheat_seeds", 1)
            } else {
                return None;
            }
        }
        "dead_bush" if shears => (name, 1),
        "dead_bush" => ("stick", sample(random, 0, 2)),
        "cobweb" => (if shears { "cobweb" } else { "string" }, 1),
        "vine" | "glow_lichen" => {
            if shears {
                (name, 1)
            } else {
                return None;
            }
        }
        _ if name.ends_with("_leaves") && shears => (name, 1),
        "stone" => ("cobblestone", 1),
        "deepslate" => ("cobbled_deepslate", 1),
        "grass_block" | "podzol" | "mycelium" | "dirt_path" | "farmland" => ("dirt", 1),
        "coal_ore" | "deepslate_coal_ore" => ("coal", 1),
        "iron_ore" | "deepslate_iron_ore" => ("raw_iron", 1),
        "gold_ore" | "deepslate_gold_ore" => ("raw_gold", 1),
        "copper_ore" | "deepslate_copper_ore" => ("raw_copper", sample(random, 2, 5)),
        "diamond_ore" | "deepslate_diamond_ore" => ("diamond", 1),
        "emerald_ore" | "deepslate_emerald_ore" => ("emerald", 1),
        "lapis_ore" | "deepslate_lapis_ore" => ("lapis_lazuli", sample(random, 4, 9)),
        "redstone_ore" | "deepslate_redstone_ore" => ("redstone", sample(random, 4, 5)),
        "nether_quartz_ore" => ("quartz", 1),
        "nether_gold_ore" => ("gold_nugget", sample(random, 2, 6)),
        "gravel" if sample(random, 0, 9) == 0 => ("flint", 1),
        "glass" | "glass_pane" | "ice" | "frosted_ice" | "spawner" | "trial_spawner" => {
            return None
        }
        _ if name.ends_with("_leaves")
            || name.ends_with("_glass")
            || name.ends_with("_glass_pane") =>
        {
            return None
        }
        _ => {
            // Stateful crop/multiblock loot requires a dedicated contract.
            let metadata = block_state::block_metadata(name)?;
            if metadata
                .properties
                .iter()
                .any(|property| property.name == "age")
                || property_value(metadata, state.as_u32(), "half") == Some("upper")
                || matches!(
                    name,
                    "snow" | "fire" | "soul_fire" | "wheat" | "carrots" | "potatoes" | "beetroots"
                )
            {
                return None;
            }
            (name, 1)
        }
    };
    if count == 0 {
        return None;
    }
    ItemStack::try_new(ItemId::from_name(drop)?, count, ComponentPatch::empty()).ok()
}

pub(crate) fn ore_experience(state: crate::BlockStateId, random: &mut u64) -> u32 {
    let Some(name) = block_state::state_id_to_block_name(state.as_u32()) else {
        return 0;
    };
    u32::from(match name {
        "coal_ore" | "deepslate_coal_ore" => sample(random, 0, 2),
        "diamond_ore" | "deepslate_diamond_ore" | "emerald_ore" | "deepslate_emerald_ore" => {
            sample(random, 3, 7)
        }
        "lapis_ore" | "deepslate_lapis_ore" | "nether_quartz_ore" => sample(random, 2, 5),
        "redstone_ore" | "deepslate_redstone_ore" => sample(random, 1, 5),
        "nether_gold_ore" => sample(random, 0, 1),
        _ => 0,
    })
}

fn sample(random: &mut u64, minimum: u8, maximum: u8) -> u8 {
    *random ^= *random << 13;
    *random ^= *random >> 7;
    *random ^= *random << 17;
    minimum + (*random % u64::from(maximum - minimum + 1)) as u8
}

pub(crate) fn property_value(
    metadata: &block_state::BlockMetadata,
    state: u32,
    name: &str,
) -> Option<&'static str> {
    let index = metadata
        .properties
        .iter()
        .position(|property| property.name == name)?;
    let multiplier: usize = metadata.properties[index + 1..]
        .iter()
        .map(|property| property.cardinality)
        .product();
    let property = &metadata.properties[index];
    let digit = usize::try_from(state.checked_sub(metadata.min_state)?).ok()? / multiplier
        % property.cardinality;
    property.values.get(digit).copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tool(name: &str) -> MiningTool {
        MiningTool::new(Some(ItemId::from_name(name).unwrap().id()), true, false).unwrap()
    }
    fn state(name: &str) -> crate::BlockStateId {
        crate::BlockStateId::new(block_state::block_default_state(name).unwrap())
    }
    #[test]
    fn mining_profiles_respect_tiers_and_environment() {
        assert!(
            tool("wooden_pickaxe")
                .profile(state("coal_ore"))
                .unwrap()
                .correct
        );
        assert!(
            !tool("wooden_pickaxe")
                .profile(state("iron_ore"))
                .unwrap()
                .correct
        );
        assert!(
            tool("stone_pickaxe")
                .profile(state("iron_ore"))
                .unwrap()
                .correct
        );
        assert!(
            !tool("stone_pickaxe")
                .profile(state("diamond_ore"))
                .unwrap()
                .correct
        );
        assert!(
            tool("iron_pickaxe")
                .profile(state("diamond_ore"))
                .unwrap()
                .correct
        );
        assert!(
            !tool("golden_pickaxe")
                .profile(state("diamond_ore"))
                .unwrap()
                .correct
        );
        let profile = tool("wooden_pickaxe").profile(state("stone")).unwrap();
        assert_eq!(profile.progress, 2.0 / 1.5 / 30.0);
        let airborne = MiningTool::new(tool("wooden_pickaxe").held_item(), false, true).unwrap();
        assert_eq!(
            airborne.profile(state("stone")).unwrap().progress,
            2.0_f32 / 5.0 / 5.0 / 1.5 / 30.0
        );
        assert!(MiningTool::empty().profile(state("bedrock")).is_none());
    }
    #[test]
    fn mining_drops_match_ordinary_progression_and_pinned_ranges() {
        let mut random = 1;
        assert_eq!(
            ordinary_drop(state("oak_log"), &mut random).unwrap().item(),
            ItemId::from_name("oak_log")
        );
        assert_eq!(
            ordinary_drop(state("stone"), &mut random).unwrap().item(),
            ItemId::from_name("cobblestone")
        );
        for _ in 0..64 {
            let drop = ordinary_drop(state("copper_ore"), &mut random).unwrap();
            assert_eq!(drop.item(), ItemId::from_name("raw_copper"));
            assert!((2..=5).contains(&drop.count()));
            assert!((3..=7).contains(&ore_experience(state("diamond_ore"), &mut random)));
        }
    }

    #[test]
    fn mining_original_special_tools_and_zero_hardness_rules() {
        let shears = tool("shears");
        assert_eq!(
            shears.profile(state("white_wool")).unwrap().progress,
            5.0 / 0.8 / 30.0
        );
        assert_eq!(
            shears.profile(state("vine")).unwrap().progress,
            2.0 / 0.2 / 30.0
        );
        assert_eq!(
            tool("wooden_sword")
                .profile(state("oak_leaves"))
                .unwrap()
                .progress,
            1.5 / 0.2 / 30.0
        );
        assert!(
            tool("wooden_sword")
                .profile(state("bamboo"))
                .unwrap()
                .progress
                >= 1.0
        );
        assert!(
            !MiningTool::empty()
                .profile(state("cobweb"))
                .unwrap()
                .correct
        );
        assert!(
            !tool("diamond_pickaxe")
                .profile(state("cobweb"))
                .unwrap()
                .correct
        );
        for name in ["wooden_sword", "shears"] {
            let profile = tool(name).profile(state("cobweb")).unwrap();
            assert!(profile.correct);
            assert_eq!(profile.progress, 15.0 / 4.0 / 30.0);
        }
        for grounded in [false, true] {
            for submerged in [false, true] {
                let bare = MiningTool::new(None, grounded, submerged)
                    .unwrap()
                    .profile(state("short_grass"))
                    .unwrap();
                assert!(bare.progress.is_infinite());
                assert_eq!(bare.tool_damage, 0);
                let sheared = shears
                    .with_environment(grounded, submerged)
                    .profile(state("short_grass"))
                    .unwrap();
                assert!(sheared.progress.is_infinite());
                assert_eq!(sheared.tool_damage, 1);
            }
        }
    }

    #[test]
    fn mining_original_tool_conditioned_plant_and_web_loot() {
        let mut random = 1;
        assert!(mined_drop(
            state("short_grass"),
            MiningTool::empty(),
            false,
            &mut random
        )
        .is_none());
        random = 8;
        assert_eq!(
            mined_drop(state("fern"), MiningTool::empty(), false, &mut random)
                .unwrap()
                .item(),
            ItemId::from_name("wheat_seeds")
        );
        let unchanged = random;
        for (name, expected, count, paired) in [
            ("short_grass", "short_grass", 1, false),
            ("fern", "fern", 1, false),
            ("tall_grass", "short_grass", 2, true),
            ("large_fern", "fern", 2, true),
            ("oak_leaves", "oak_leaves", 1, false),
            ("vine", "vine", 1, false),
            ("glow_lichen", "glow_lichen", 1, false),
            ("dead_bush", "dead_bush", 1, false),
            ("cobweb", "cobweb", 1, false),
        ] {
            let drop = mined_drop(state(name), tool("shears"), paired, &mut random).unwrap();
            assert_eq!(drop.item(), ItemId::from_name(expected));
            assert_eq!(drop.count(), count);
            assert_eq!(
                random, unchanged,
                "shears take the first alternative without an RNG roll"
            );
        }
        assert!(mined_drop(state("tall_grass"), tool("shears"), false, &mut random).is_none());
        assert_eq!(random, unchanged);
        assert_eq!(
            mined_drop(state("cobweb"), tool("wooden_sword"), false, &mut random)
                .unwrap()
                .item(),
            ItemId::from_name("string")
        );
        assert!(mined_drop(state("vine"), MiningTool::empty(), false, &mut random).is_none());
        assert!(mined_drop(
            state("glow_lichen"),
            MiningTool::empty(),
            false,
            &mut random
        )
        .is_none());
        let mut counts = [false; 3];
        for _ in 0..64 {
            let count = mined_drop(state("dead_bush"), MiningTool::empty(), false, &mut random)
                .map_or(0, |s| {
                    assert_eq!(s.item(), ItemId::from_name("stick"));
                    s.count()
                });
            counts[usize::from(count)] = true;
        }
        assert_eq!(counts, [true; 3]);
    }
}
