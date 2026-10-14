//! Strict interaction-tail decoding and immutable ordinary equipment facts.
//! Target identity, reach/cooldown, live vitals, accepted wear receipts, events
//! and all world/player mutations remain with their exclusive runtime owners.

use ferrumc_codec::BoundedReader;
use ferrumc_core::GameMode;
use ferrumc_items::{
    armor_properties, durability, stack_capacity, tool_properties, ItemRuleError, ItemStack,
};
use ferrumc_math::Vec3;
use ferrumc_proto::generated::play::InteractEntity;

use crate::inventory::PlayerInventory;

/// The original packet's two admitted interaction hands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InteractionHand {
    /// Original enum index zero.
    Main,
    /// Original enum index one.
    Off,
}

/// A completely decoded bounded action; no opaque unparsed bytes survive.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Interaction {
    /// Main-hand attack, with the independent secondary-use flag.
    Attack { secondary: bool },
    /// Ordinary use with the admitted hand.
    Interact {
        hand: InteractionHand,
        secondary: bool,
    },
    /// Use at a finite entity-relative hit position, preserving original f32s.
    InteractAt {
        hand: InteractionHand,
        location: Vec3,
        secondary: bool,
    },
}

impl Interaction {
    /// Packet's secondary-action flag; separate from action and hand enums.
    pub(crate) fn secondary(self) -> bool {
        match self {
            Self::Attack { secondary }
            | Self::Interact { secondary, .. }
            | Self::InteractAt { secondary, .. } => secondary,
        }
    }

    /// Whether the action grants only a request to the exclusive attack owner.
    pub(crate) fn is_attack(self) -> bool {
        matches!(self, Self::Attack { .. })
    }
}

/// Decode original action0 hand+bool, action1 bool, action2 xyz+hand+bool.
/// Hand/action enums and bools are strict, all hit coordinates must be finite,
/// and every action must exhaust its exact bounded tail before admission. Target
/// ids remain caller-owned identities; parsing grants no entity/reach authority.
/// Bool0/1 and finite-hit admission are conservative `FerrumC` bounds beyond the
/// original `FriendlyByteBuf` reader's broader nonzero-bool/nonfinite acceptance.
pub(crate) fn decode_interaction(packet: &InteractEntity) -> anyhow::Result<Interaction> {
    let mut reader = BoundedReader::new(packet.interaction_raw().as_slice());
    let interaction = match packet.action() {
        0 => {
            let hand = read_hand(&mut reader)?;
            Interaction::Interact {
                hand,
                secondary: read_secondary(&mut reader)?,
            }
        }
        1 => Interaction::Attack {
            secondary: read_secondary(&mut reader)?,
        },
        2 => {
            let x = reader.read_f32()?;
            let y = reader.read_f32()?;
            let z = reader.read_f32()?;
            anyhow::ensure!(
                x.is_finite() && y.is_finite() && z.is_finite(),
                "nonfinite entity interaction location"
            );
            let hand = read_hand(&mut reader)?;
            Interaction::InteractAt {
                hand,
                location: Vec3::new(f64::from(x), f64::from(y), f64::from(z)),
                secondary: read_secondary(&mut reader)?,
            }
        }
        action => anyhow::bail!("invalid entity interaction action {action}"),
    };
    reader.finish()?;
    Ok(interaction)
}

fn read_hand(reader: &mut BoundedReader<'_>) -> anyhow::Result<InteractionHand> {
    match reader.read_var_int()? {
        0 => Ok(InteractionHand::Main),
        1 => Ok(InteractionHand::Off),
        hand => anyhow::bail!("invalid entity interaction hand {hand}"),
    }
}

fn read_secondary(reader: &mut BoundedReader<'_>) -> anyhow::Result<bool> {
    match reader.read_u8()? {
        0 => Ok(false),
        1 => Ok(true),
        value => anyhow::bail!("invalid entity interaction secondary flag {value}"),
    }
}

/// Fixed-size ordinary combat attributes, with no health or mutable stack copy.
/// Profiles grant no identity, hit acceptance, cooldown or durability authority.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CombatProfile {
    attack_damage: f32,
    attack_speed: f32,
    attack_wear: u32,
    armor: f32,
    toughness: f32,
}

impl CombatProfile {
    /// Resolve the currently selected main hand and actual armor positions5..8.
    /// Ordinary item tables supply original defaults; only matching equipment
    /// slots contribute armor. Relevant complex overrides refuse rather than
    /// silently dropping their semantics. Known removals use Items admission.
    /// Wear is the successful-hit cost, suppressed for creative, unbreakable or
    /// absent damageability. No inventory stack, counter or health is mutated.
    pub(crate) fn from_inventory(inventory: &PlayerInventory) -> anyhow::Result<Self> {
        let held = inventory.held();
        admit_semantics(held)?;
        let held_durability = durability(held)?;
        let mut profile = Self {
            attack_damage: 1.0,
            attack_speed: 4.0,
            attack_wear: 0,
            armor: 0.0,
            toughness: 0.0,
        };
        if let Some(tool) = tool_properties(held)? {
            profile.attack_damage = tool.attack_damage();
            profile.attack_speed = tool.attack_speed();
            if inventory.game_mode() != GameMode::Creative
                && held_durability.is_some_and(|state| !state.unbreakable())
            {
                profile.attack_wear = tool.attack_durability_cost();
            }
        }
        for index in 5..=8 {
            let stack = inventory
                .slot(index)
                .ok_or_else(|| anyhow::anyhow!("missing armor inventory slot"))?;
            admit_semantics(stack)?;
            durability(stack)?;
            if let Some(armor) = armor_properties(stack)? {
                if armor.inventory_slot() == index {
                    profile.armor += armor.armor();
                    profile.toughness += armor.toughness();
                }
            }
        }
        // This also verifies the original attribute ranges consumed by Sim.
        ferrumc_items::damage_after_armor(0.0, profile.armor, profile.toughness)?;
        anyhow::ensure!(
            profile.attack_damage.is_finite()
                && profile.attack_damage >= 0.0
                && profile.attack_speed.is_finite()
                && profile.attack_speed > 0.0,
            "invalid ordinary attack attributes"
        );
        Ok(profile)
    }

    /// Base player damage plus the original selected main-hand item modifier.
    pub(crate) fn attack_damage(self) -> f32 {
        self.attack_damage
    }
    /// Original base speed4 plus item modifier; owner applies attack tick policy.
    pub(crate) fn attack_speed(self) -> f32 {
        self.attack_speed
    }
    /// Effective durability cost after a successful accepted ordinary attack.
    pub(crate) fn attack_wear(self) -> u32 {
        self.attack_wear
    }
    /// Total compatible original equipped armor attribute.
    pub(crate) fn armor(self) -> f32 {
        self.armor
    }
    /// Total compatible original equipped toughness attribute.
    pub(crate) fn toughness(self) -> f32 {
        self.toughness
    }
}

fn admit_semantics(stack: &ItemStack) -> anyhow::Result<()> {
    stack_capacity(stack)?;
    // Enchantments can affect damage, attributes, protection and probabilistic
    // wear. None of those complex additions may masquerade as vanilla defaults.
    // Items helpers interpret scalar durability and known removals separately.
    for value in stack.components().added() {
        if matches!(value.type_id(), 10 | 13 | 25 | 26 | 28) {
            return Err(ItemRuleError::UnsupportedComponent {
                id: value.type_id(),
            }
            .into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU8;

    use bytes::BytesMut;
    use ferrumc_codec::BoundedBytes;
    use ferrumc_items::{ComponentPatch, ComponentTypeId, ComponentValue, ItemId, OpaqueComponent};

    use super::*;
    use crate::inventory::SLOT_COUNT;

    // Full packet bytes independently serialized and decoded by installed
    // minecraft-protocol1.66.2/minecraft-data3.111.0 for protocol772/1.21.8.
    const WIRE_GOLDENS: [&str; 6] = [
        "19250100",
        "19250101",
        "1925000000",
        "1925000101",
        "1925023fa0000080000000c02000000001",
        "192502c0600000408800003f0000000100",
    ];

    fn hex_bytes(value: &str) -> Vec<u8> {
        (0..value.len())
            .step_by(2)
            .map(|index| u8::from_str_radix(&value[index..index + 2], 16).unwrap())
            .collect()
    }

    fn decode_wire(bytes: &[u8]) -> anyhow::Result<(InteractEntity, Interaction)> {
        let mut reader = BoundedReader::new(bytes);
        anyhow::ensure!(reader.read_var_int()? == InteractEntity::PACKET_ID);
        let packet = InteractEntity::decode(&mut reader)?;
        reader.finish()?;
        let interaction = decode_interaction(&packet)?;
        Ok((packet, interaction))
    }

    fn packet(action: i32, tail: &[u8]) -> InteractEntity {
        InteractEntity::new(37, action, BoundedBytes::new(tail.to_vec()).unwrap())
    }

    fn hit_tail(coordinates: [f32; 3], hand: u8, secondary: u8) -> Vec<u8> {
        let mut tail = Vec::new();
        for value in coordinates {
            tail.extend_from_slice(&value.to_be_bytes());
        }
        tail.extend_from_slice(&[hand, secondary]);
        tail
    }

    fn stack(name: &str, components: ComponentPatch) -> ItemStack {
        ItemStack::new(ItemId::from_name(name).unwrap(), NonZeroU8::MIN, components)
    }

    fn ordinary(name: &str) -> ItemStack {
        stack(name, ComponentPatch::empty())
    }

    fn inventory(mode: GameMode, held: ItemStack) -> PlayerInventory {
        let mut slots = std::array::from_fn(|_| ItemStack::empty());
        slots[36] = held;
        PlayerInventory::from_persisted(mode, 0, slots, ItemStack::empty())
    }

    fn with_removed(name: &str, ids: &[i32]) -> ItemStack {
        stack(
            name,
            ComponentPatch::new(
                Vec::new(),
                ids.iter().copied().map(ComponentTypeId::new).collect(),
            ),
        )
    }

    fn opaque(id: i32, raw: &[u8]) -> ComponentValue {
        ComponentValue::Opaque(OpaqueComponent::new(id, raw.to_vec()).unwrap())
    }

    fn profile_bits(profile: CombatProfile) -> [u32; 5] {
        [
            profile.attack_damage().to_bits(),
            profile.attack_speed().to_bits(),
            profile.attack_wear(),
            profile.armor().to_bits(),
            profile.toughness().to_bits(),
        ]
    }

    #[test]
    fn independent_wire_goldens_decode_all_actions_and_both_hands() {
        let expected = [
            Interaction::Attack { secondary: false },
            Interaction::Attack { secondary: true },
            Interaction::Interact {
                hand: InteractionHand::Main,
                secondary: false,
            },
            Interaction::Interact {
                hand: InteractionHand::Off,
                secondary: true,
            },
            Interaction::InteractAt {
                hand: InteractionHand::Main,
                location: Vec3::new(1.25, -0.0, -2.5),
                secondary: true,
            },
            Interaction::InteractAt {
                hand: InteractionHand::Off,
                location: Vec3::new(-3.5, 4.25, 0.5),
                secondary: false,
            },
        ];
        for (golden, expected) in WIRE_GOLDENS.into_iter().zip(expected) {
            let bytes = hex_bytes(golden);
            let (packet, interaction) = decode_wire(&bytes).unwrap();
            assert_eq!(packet.target(), 37);
            assert_eq!(interaction, expected);
            assert_eq!(interaction.secondary(), expected.secondary());
            assert_eq!(
                interaction.is_attack(),
                matches!(expected, Interaction::Attack { .. })
            );
            let mut encoded = BytesMut::new();
            packet.encode(&mut encoded).unwrap();
            assert_eq!(&encoded[..], &bytes);
        }
        let (_, Interaction::InteractAt { location, .. }) =
            decode_wire(&hex_bytes(WIRE_GOLDENS[4])).unwrap()
        else {
            panic!("interact-at golden");
        };
        assert_eq!(location.y.to_bits(), (-0.0_f64).to_bits());
    }

    #[test]
    fn every_wire_prefix_and_trailing_byte_refuses() {
        for golden in WIRE_GOLDENS {
            let bytes = hex_bytes(golden);
            for length in 0..bytes.len() {
                assert!(decode_wire(&bytes[..length]).is_err(), "{golden}/{length}");
            }
            for trailing in 0..=u8::MAX {
                let mut extended = bytes.clone();
                extended.push(trailing);
                assert!(decode_wire(&extended).is_err(), "{golden}+{trailing}");
            }
        }
        let mut oversized = vec![0x19, 37, 1];
        oversized.extend([0; 19]);
        assert!(decode_wire(&oversized).is_err());
    }

    #[test]
    fn invalid_actions_hands_flags_and_unterminated_varints_refuse() {
        for action in [-1, 3, i32::MIN, i32::MAX] {
            assert!(decode_interaction(&packet(action, &[0])).is_err());
        }
        for hand in [2, 3, 127] {
            assert!(decode_interaction(&packet(0, &[hand, 0])).is_err());
            assert!(decode_interaction(&packet(2, &hit_tail([0.0; 3], hand, 0))).is_err());
        }
        for hand in [
            vec![0xff, 0xff, 0xff, 0xff, 0x0f],
            vec![0x80; 5],
            vec![0x80; 6],
        ] {
            let mut tail = hand;
            tail.push(0);
            assert!(decode_interaction(&packet(0, &tail)).is_err());
            let mut at_tail = hit_tail([0.0; 3], 0, 0);
            at_tail.truncate(12);
            at_tail.extend_from_slice(&tail);
            if at_tail.len() <= 18 {
                assert!(decode_interaction(&packet(2, &at_tail)).is_err());
            }
        }
        for flag in [2, 127, 255] {
            assert!(decode_interaction(&packet(1, &[flag])).is_err());
            for hand in [0, 1] {
                assert!(decode_interaction(&packet(0, &[hand, flag])).is_err());
                assert!(decode_interaction(&packet(2, &hit_tail([0.0; 3], hand, flag))).is_err());
            }
        }
    }

    #[test]
    fn hand_varints_retain_original_resolved_value_policy() {
        // Actual original VarInt.java, like the shared reader, uses Java32-bit
        // shifting: high fifth-byte bits truncate, and nonminimal forms are legal.
        for (tail, hand) in [
            (vec![0x80, 0, 1], InteractionHand::Main),
            (vec![0x81, 0, 0], InteractionHand::Off),
            (vec![0x80, 0x80, 0x80, 0x80, 0x10, 0], InteractionHand::Main),
            (vec![0x81, 0x80, 0x80, 0x80, 0x70, 0], InteractionHand::Off),
        ] {
            let action = decode_interaction(&packet(0, &tail)).unwrap();
            assert!(matches!(action, Interaction::Interact { hand: actual, .. } if actual == hand));
        }
    }

    #[test]
    fn nonfinite_hit_coordinates_refuse_and_finite_bits_survive() {
        for index in 0..3 {
            for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                let mut coordinates = [0.0; 3];
                coordinates[index] = value;
                assert!(decode_interaction(&packet(2, &hit_tail(coordinates, 0, 0))).is_err());
            }
        }
        for coordinates in [
            [f32::MIN, -0.0, f32::MAX],
            [f32::from_bits(1), 0.0, -f32::from_bits(1)],
        ] {
            let Interaction::InteractAt { location, .. } =
                decode_interaction(&packet(2, &hit_tail(coordinates, 1, 1))).unwrap()
            else {
                panic!("interact-at");
            };
            assert_eq!(
                [
                    location.x.to_bits(),
                    location.y.to_bits(),
                    location.z.to_bits()
                ],
                coordinates.map(|value| f64::from(value).to_bits())
            );
        }
    }

    #[test]
    fn bare_hand_and_nonweapon_use_original_player_defaults() {
        for held in [
            ItemStack::empty(),
            ordinary("minecraft:stone"),
            ordinary("minecraft:apple"),
        ] {
            let profile =
                CombatProfile::from_inventory(&inventory(GameMode::Survival, held)).unwrap();
            assert_eq!(
                profile_bits(profile),
                [1.0_f32.to_bits(), 4.0_f32.to_bits(), 0, 0, 0]
            );
        }
        assert_eq!(std::mem::size_of::<CombatProfile>(), 20);
    }

    #[test]
    fn selected_main_hand_uses_original_weapon_attributes_and_costs() {
        for (name, damage, speed_bits, wear) in [
            ("minecraft:diamond_sword", 7.0_f32, 0x3fcc_cccc, 1),
            ("minecraft:diamond_axe", 9.0, 0x3f80_0000, 2),
            ("minecraft:diamond_pickaxe", 5.0, 0x3f99_999a, 2),
            ("minecraft:diamond_shovel", 5.5, 0x3f80_0000, 2),
            ("minecraft:netherite_axe", 10.0, 0x3f80_0000, 2),
            ("minecraft:mace", 6.0, 0x3f19_9998, 1),
            ("minecraft:trident", 9.0, 0x3f8c_cccc, 1),
        ] {
            let mut inventory = inventory(GameMode::Survival, ordinary("minecraft:stone"));
            *inventory.slot_mut(44).unwrap() = ordinary(name);
            *inventory.slot_mut(45).unwrap() = ordinary("minecraft:netherite_sword");
            assert_eq!(
                CombatProfile::from_inventory(&inventory)
                    .unwrap()
                    .attack_damage()
                    .to_bits(),
                1.0_f32.to_bits()
            );
            assert!(inventory.set_selected(8));
            assert_eq!(
                profile_bits(CombatProfile::from_inventory(&inventory).unwrap()),
                [damage.to_bits(), speed_bits, wear, 0, 0],
                "{name}"
            );
        }
    }

    #[test]
    fn only_compatible_armor_positions_contribute() {
        for (material, toughness) in [("diamond", 8.0_f32), ("netherite", 12.0)] {
            let mut inventory = inventory(GameMode::Survival, ItemStack::empty());
            for (slot, piece) in [
                (5, "helmet"),
                (6, "chestplate"),
                (7, "leggings"),
                (8, "boots"),
            ] {
                *inventory.slot_mut(slot).unwrap() =
                    ordinary(&format!("minecraft:{material}_{piece}"));
            }
            let profile = CombatProfile::from_inventory(&inventory).unwrap();
            assert_eq!(profile.armor().to_bits(), 20.0_f32.to_bits());
            assert_eq!(profile.toughness().to_bits(), toughness.to_bits());
        }
        let mut inventory = inventory(GameMode::Survival, ordinary("minecraft:diamond_chestplate"));
        *inventory.slot_mut(5).unwrap() = ordinary("minecraft:diamond_chestplate");
        *inventory.slot_mut(6).unwrap() = ordinary("minecraft:diamond_helmet");
        *inventory.slot_mut(45).unwrap() = ordinary("minecraft:diamond_leggings");
        let profile = CombatProfile::from_inventory(&inventory).unwrap();
        assert_eq!(profile.armor().to_bits(), 0);
        assert_eq!(profile.toughness().to_bits(), 0);
    }

    #[test]
    fn known_removals_follow_items_semantics() {
        let mut inventory = inventory(
            GameMode::Survival,
            with_removed("minecraft:diamond_sword", &[13]),
        );
        assert_eq!(
            profile_bits(CombatProfile::from_inventory(&inventory).unwrap()),
            [1.0_f32.to_bits(), 4.0_f32.to_bits(), 1, 0, 0]
        );
        *inventory.slot_mut(36).unwrap() = with_removed("minecraft:diamond_sword", &[25, 26]);
        assert_eq!(
            CombatProfile::from_inventory(&inventory)
                .unwrap()
                .attack_wear(),
            0
        );
        *inventory.slot_mut(6).unwrap() = with_removed("minecraft:diamond_chestplate", &[13]);
        *inventory.slot_mut(7).unwrap() = with_removed("minecraft:diamond_leggings", &[28]);
        let profile = CombatProfile::from_inventory(&inventory).unwrap();
        assert_eq!(profile.armor().to_bits(), 0);
        assert_eq!(profile.toughness().to_bits(), 0);
    }

    #[test]
    fn wear_policy_suppresses_creative_unbreakable_and_missing_damageability() {
        for mode in [
            GameMode::Survival,
            GameMode::Adventure,
            GameMode::Creative,
            GameMode::Spectator,
        ] {
            let profile = CombatProfile::from_inventory(&inventory(
                mode,
                ordinary("minecraft:diamond_sword"),
            ))
            .unwrap();
            assert_eq!(profile.attack_wear(), u32::from(mode != GameMode::Creative));
        }
        for components in [
            ComponentPatch::new(vec![ComponentValue::Unbreakable], Vec::new()),
            ComponentPatch::new(vec![opaque(4, &[])], Vec::new()),
            ComponentPatch::new(Vec::new(), vec![ComponentTypeId::new(2)]),
            ComponentPatch::new(Vec::new(), vec![ComponentTypeId::new(3)]),
        ] {
            let profile = CombatProfile::from_inventory(&inventory(
                GameMode::Survival,
                stack("minecraft:diamond_sword", components),
            ))
            .unwrap();
            assert_eq!(profile.attack_wear(), 0);
            assert_eq!(profile.attack_damage().to_bits(), 7.0_f32.to_bits());
        }
    }

    #[test]
    fn complex_combat_additions_refuse_on_every_relevant_position() {
        for id in [10, 13, 25, 26, 28] {
            for slot in [5, 6, 7, 8, 36] {
                let mut inventory = inventory(GameMode::Survival, ItemStack::empty());
                *inventory.slot_mut(slot).unwrap() = stack(
                    "minecraft:stone",
                    ComponentPatch::new(vec![opaque(id, &[0])], Vec::new()),
                );
                let error = CombatProfile::from_inventory(&inventory).unwrap_err();
                assert!(
                    matches!(error.downcast_ref::<ItemRuleError>(), Some(ItemRuleError::UnsupportedComponent { id: actual }) if *actual == id),
                    "id{id} slot{slot}"
                );
            }
        }
    }

    #[test]
    fn malformed_stack_capacity_and_scalar_components_refuse_in_every_mode() {
        let bad_stacks = [
            ItemStack::new(
                ItemId::from_name("minecraft:diamond_sword").unwrap(),
                NonZeroU8::new(2).unwrap(),
                ComponentPatch::empty(),
            ),
            ordinary("minecraft:air"),
            stack(
                "minecraft:stone",
                ComponentPatch::new(vec![opaque(1, &[0])], Vec::new()),
            ),
            stack(
                "minecraft:diamond_sword",
                ComponentPatch::new(vec![ComponentValue::MaxDamage(0)], Vec::new()),
            ),
            stack(
                "minecraft:diamond_sword",
                ComponentPatch::new(vec![opaque(3, &[0x80])], Vec::new()),
            ),
            stack(
                "minecraft:stone",
                ComponentPatch::new(vec![opaque(4, &[1])], Vec::new()),
            ),
        ];
        for mode in [GameMode::Survival, GameMode::Creative] {
            for bad in &bad_stacks {
                for slot in [5, 6, 7, 8, 36] {
                    let mut inventory = inventory(mode, ItemStack::empty());
                    *inventory.slot_mut(slot).unwrap() = bad.clone();
                    assert!(
                        CombatProfile::from_inventory(&inventory).is_err(),
                        "{mode:?}/{slot}/{bad:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn profile_preserves_inventory_and_unrelated_component_encoding() {
        let held = stack(
            "minecraft:diamond_sword",
            ComponentPatch::new(
                vec![opaque(42, &[0x80, 0x00]), ComponentValue::Damage(-4)],
                vec![ComponentTypeId::new(6)],
            ),
        );
        let mut inventory = inventory(GameMode::Survival, held);
        *inventory.slot_mut(6).unwrap() = stack(
            "minecraft:diamond_chestplate",
            ComponentPatch::new(vec![ComponentValue::Damage(99)], Vec::new()),
        );
        let before = inventory.container_content_payload().unwrap();
        let slots_before: Vec<_> = (0..SLOT_COUNT)
            .map(|index| {
                ferrumc_items::encode_persisted_stack(inventory.slot(index).unwrap()).unwrap()
            })
            .collect();
        let state_id = inventory.state_id();
        let first = CombatProfile::from_inventory(&inventory).unwrap();
        let second = CombatProfile::from_inventory(&inventory).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.attack_wear(), 1);
        assert_eq!(inventory.state_id(), state_id);
        assert_eq!(inventory.selected(), 0);
        assert_eq!(inventory.container_content_payload().unwrap(), before);
        for (index, original) in slots_before.iter().enumerate() {
            assert_eq!(
                &ferrumc_items::encode_persisted_stack(inventory.slot(index).unwrap()).unwrap(),
                original
            );
        }
    }
}
