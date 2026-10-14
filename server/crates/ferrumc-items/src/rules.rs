//! Checked ordinary food, durability, equipment and attack defaults for 1.21.8.
//!
//! The closed catalog is generated from the pinned registry snapshot and read
//! against complete Paper build 60 Foods/Consumables/Items/ToolMaterial/ArmorMaterial
//! sources. Helpers plan data only; consumers own timing, effects, enchanting,
//! game-mode authority, inventory publication, events and durability commits.
mod data;
#[cfg(test)]
mod tests;

use ferrumc_codec::BoundedReader;

use crate::{
    component, persisted_stack_size, ComponentPatch, ComponentValue, ItemId, ItemPersistenceError,
    ItemStack,
};

const FOOD: i32 = 20;
const CONSUMABLE: i32 = 21;
const USE_REMAINDER: i32 = 22;
const EQUIPPABLE: i32 = 28;
const ATTRIBUTE_MODIFIERS: i32 = 13;
const ENCHANTMENTS: i32 = 10;
const SUSPICIOUS_STEW_EFFECTS: i32 = 44;

/// A gameplay rule refusal that never strips or rewrites the source stack.
#[derive(Debug, thiserror::Error)]
pub enum ItemRuleError {
    /// The authoritative stack failed lossless persistence admission.
    #[error(transparent)]
    InvalidStack(#[from] ItemPersistenceError),
    /// A recognized scalar component has an invalid payload or trailing bytes.
    #[error("malformed item rule component {id}")]
    MalformedComponent {
        /// Pinned protocol component id.
        id: i32,
    },
    /// This ordinary consumer cannot interpret a complex component override.
    #[error("unsupported item rule component override {id}")]
    UnsupportedComponent {
        /// Pinned protocol component id.
        id: i32,
    },
    /// Present-air remains distinct from canonical empty at gameplay boundaries.
    #[error("present-air item has no gameplay rules")]
    PresentAir,
    /// Damage or armor attributes are nonfinite or outside their original ranges.
    #[error("invalid damage or armor attributes")]
    InvalidCombatInput,
}

/// One builtin food's non-nutrition consume action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoodEffectKind {
    /// Adds hunger exhaustion over its duration.
    Hunger,
    /// Poisons without delivering a killing blow.
    Poison,
    /// Adds the nausea status.
    Nausea,
    /// Regenerates health over its duration.
    Regeneration,
    /// Reduces incoming damage.
    Resistance,
    /// Suppresses fire damage.
    FireResistance,
    /// Adds temporary absorption hearts.
    Absorption,
    /// Honey removes poison.
    ClearPoison,
    /// Chorus fruit attempts a bounded random teleport.
    RandomTeleport,
}

/// A builtin consume-effect descriptor; callers own deterministic RNG and ticks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FoodEffect {
    kind: FoodEffectKind,
    ticks: u32,
    amplifier: u8,
    probability: f32,
}
impl FoodEffect {
    /// The effect action.
    pub fn kind(self) -> FoodEffectKind {
        self.kind
    }
    /// Duration in simulation ticks (zero for an instantaneous action).
    pub fn ticks(self) -> u32 {
        self.ticks
    }
    /// Zero-based status strength.
    pub fn amplifier(self) -> u8 {
        self.amplifier
    }
    /// Probability per consume action, from the builtin consumable.
    pub fn probability(self) -> f32 {
        self.probability
    }
}

/// Resolved ordinary food values plus the original consumable defaults.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FoodProperties {
    nutrition: i32,
    saturation: f32,
    always_edible: bool,
    consume_ticks: u16,
    remainder_name: Option<&'static str>,
    effects: &'static [FoodEffect],
}
impl FoodProperties {
    /// Hunger restored; consumers must add with overflow protection and cap at 20.
    pub fn nutrition(self) -> i32 {
        self.nutrition
    }
    /// Absolute saturation gain, already multiplied by nutrition and two.
    pub fn saturation(self) -> f32 {
        self.saturation
    }
    /// Whether consumption may start at full hunger.
    pub fn always_edible(self) -> bool {
        self.always_edible
    }
    /// Required continuous use duration: 32 normally, 16 for kelp, 40 for honey.
    pub fn consume_ticks(self) -> u16 {
        self.consume_ticks
    }
    /// The bowl or bottle produced after one consumed item.
    pub fn remainder(self) -> Option<ItemId> {
        self.remainder_name.and_then(ItemId::from_name)
    }
    /// Original consume effects; no effect is executed by this descriptor.
    pub fn effects(self) -> &'static [FoodEffect] {
        self.effects
    }
}

/// The player's four armor-equipment positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmorSlot {
    /// Window-0 slot 5.
    Helmet,
    /// Window-0 slot 6.
    Chestplate,
    /// Window-0 slot 7.
    Leggings,
    /// Window-0 slot 8.
    Boots,
}
impl ArmorSlot {
    /// The authoritative window-0 index used by player inventory.
    pub fn inventory_slot(self) -> usize {
        match self {
            Self::Helmet => 5,
            Self::Chestplate => 6,
            Self::Leggings => 7,
            Self::Boots => 8,
        }
    }
}

/// Original humanoid equipment attributes (including zero-defense heads/elytra).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArmorProperties {
    slot: ArmorSlot,
    armor: f32,
    toughness: f32,
    knockback_resistance: f32,
    damage_on_hurt: bool,
    swappable: bool,
}
impl ArmorProperties {
    /// Window-0 armor index.
    pub fn inventory_slot(self) -> usize {
        self.slot.inventory_slot()
    }
    /// Original equip position.
    pub fn slot(self) -> ArmorSlot {
        self.slot
    }
    /// Default armor attribute contribution.
    pub fn armor(self) -> f32 {
        self.armor
    }
    /// Default armor-toughness contribution.
    pub fn toughness(self) -> f32 {
        self.toughness
    }
    /// Default knockback-resistance contribution.
    pub fn knockback_resistance(self) -> f32 {
        self.knockback_resistance
    }
    /// Whether damage to its wearer should apply armor durability wear.
    pub fn damage_on_hurt(self) -> bool {
        self.damage_on_hurt
    }
    /// Whether item use may swap this equipment onto the wearer.
    pub fn swappable(self) -> bool {
        self.swappable
    }
}

/// Original main-hand attack attributes and tool/weapon durability costs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToolProperties {
    attack_damage: f32,
    attack_speed: f32,
    block_durability_cost: u32,
    attack_durability_cost: u32,
}
impl ToolProperties {
    /// Player base attack damage plus the original item modifier.
    pub fn attack_damage(self) -> f32 {
        self.attack_damage
    }
    /// Player base attack speed plus the original item modifier.
    pub fn attack_speed(self) -> f32 {
        self.attack_speed
    }
    /// Damage per eligible mined block; caller checks the actual block/tool rule.
    pub fn block_durability_cost(self) -> u32 {
        self.block_durability_cost
    }
    /// Damage per successful weapon attack.
    pub fn attack_durability_cost(self) -> u32 {
        self.attack_durability_cost
    }
}

/// Original defaults for one of the 40 builtin food items.
pub fn builtin_food(item: ItemId) -> Option<FoodProperties> {
    lookup(data::FOODS, item)
}
/// Original durability for all 68 builtin damageable items.
pub fn builtin_max_damage(item: ItemId) -> Option<i32> {
    lookup(data::DURABILITY, item)
}
/// Original humanoid equipment defaults; horse/wolf armor is excluded.
pub fn builtin_armor(item: ItemId) -> Option<ArmorProperties> {
    lookup(data::ARMOR, item)
}
/// Original tool/weapon attack defaults and wear costs.
pub fn builtin_tool(item: ItemId) -> Option<ToolProperties> {
    lookup(data::TOOLS, item)
}

fn lookup<T: Copy>(table: &[(&str, T)], item: ItemId) -> Option<T> {
    table
        .binary_search_by_key(&item.name(), |entry| entry.0)
        .ok()
        .map(|index| table[index].1)
}
fn admitted(stack: &ItemStack) -> Result<Option<ItemId>, ItemRuleError> {
    persisted_stack_size(stack)?;
    if stack.item().is_some_and(ItemId::is_air) {
        return Err(ItemRuleError::PresentAir);
    }
    Ok(stack.item())
}
fn removed(stack: &ItemStack, id: i32) -> bool {
    stack
        .components()
        .removed()
        .iter()
        .any(|value| value.get() == id)
}
fn addition(stack: &ItemStack, id: i32) -> Option<&ComponentValue> {
    stack
        .components()
        .added()
        .iter()
        .find(|value| value.type_id() == id)
}
fn refuse_overrides(stack: &ItemStack, ids: &[i32]) -> Result<(), ItemRuleError> {
    for id in ids {
        if addition(stack, *id).is_some() {
            return Err(ItemRuleError::UnsupportedComponent { id: *id });
        }
    }
    Ok(())
}
fn malformed(id: i32) -> ItemRuleError {
    ItemRuleError::MalformedComponent { id }
}

/// Resolves nutrition for all normal foods, respecting food/consumable removals.
///
/// FOOD's trusted scalar wire override is interpreted without allocation. Other
/// consumable/remainder/stew-effect overrides explicitly refuse this ordinary
/// consumer. Merely adding FOOD to a non-consumable does not grant consumption.
pub fn food_properties(stack: &ItemStack) -> Result<Option<FoodProperties>, ItemRuleError> {
    let Some(item) = admitted(stack)? else {
        return Ok(None);
    };
    refuse_overrides(stack, &[CONSUMABLE, USE_REMAINDER, SUSPICIOUS_STEW_EFFECTS])?;
    let Some(mut food) = builtin_food(item) else {
        return Ok(None);
    };
    if removed(stack, FOOD) || removed(stack, CONSUMABLE) {
        return Ok(None);
    }
    if removed(stack, USE_REMAINDER) {
        food.remainder_name = None;
    }
    if let Some(value) = addition(stack, FOOD) {
        let ComponentValue::Opaque(value) = value else {
            return Err(malformed(FOOD));
        };
        let mut reader = BoundedReader::new(value.raw());
        food.nutrition = reader.read_var_int().map_err(|_| malformed(FOOD))?;
        food.saturation = reader.read_f32().map_err(|_| malformed(FOOD))?;
        let always = reader.read_u8().map_err(|_| malformed(FOOD))?;
        if food.nutrition < 0
            || !food.saturation.is_finite()
            || food.saturation < 0.0
            || always > 1
            || reader.remaining() != 0
        {
            return Err(malformed(FOOD));
        }
        food.always_edible = always != 0;
    }
    Ok(Some(food))
}

/// Resolves original armor attributes, refusing unsupported semantic overrides.
/// Removing equippable prevents wearing; removing attribute modifiers yields
/// zero protection while retaining the equip position.
pub fn armor_properties(stack: &ItemStack) -> Result<Option<ArmorProperties>, ItemRuleError> {
    let Some(item) = admitted(stack)? else {
        return Ok(None);
    };
    refuse_overrides(stack, &[EQUIPPABLE, ATTRIBUTE_MODIFIERS])?;
    if removed(stack, EQUIPPABLE) {
        return Ok(None);
    }
    let Some(mut armor) = builtin_armor(item) else {
        return Ok(None);
    };
    if removed(stack, ATTRIBUTE_MODIFIERS) {
        armor.armor = 0.0;
        armor.toughness = 0.0;
        armor.knockback_resistance = 0.0;
    }
    Ok(Some(armor))
}

/// Resolves original tool/weapon attack values, refusing component overrides.
/// Attribute removal uses the ordinary player base damage 1/speed 4. Tool/weapon
/// removal removes the corresponding durability cost.
pub fn tool_properties(stack: &ItemStack) -> Result<Option<ToolProperties>, ItemRuleError> {
    let Some(item) = admitted(stack)? else {
        return Ok(None);
    };
    refuse_overrides(stack, &[ATTRIBUTE_MODIFIERS, 25, 26])?;
    let Some(mut tool) = builtin_tool(item) else {
        return Ok(None);
    };
    if removed(stack, ATTRIBUTE_MODIFIERS) {
        tool.attack_damage = 1.0;
        tool.attack_speed = 4.0;
    }
    if removed(stack, 25) {
        tool.block_durability_cost = 0;
    }
    if removed(stack, 26) {
        tool.attack_durability_cost = 0;
    }
    Ok(Some(tool))
}

/// A resolved damage/max-damage state, including unbreakable policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Durability {
    damage: i32,
    max_damage: i32,
    unbreakable: bool,
}
impl Durability {
    /// Current gameplay damage, clamped to 0..max as in ItemStack.getDamageValue.
    pub fn damage(self) -> i32 {
        self.damage
    }
    /// Positive effective maximum durability.
    pub fn max_damage(self) -> i32 {
        self.max_damage
    }
    /// Whether durability mutations must be suppressed.
    pub fn unbreakable(self) -> bool {
        self.unbreakable
    }
}
fn integer_component(value: &ComponentValue, id: i32) -> Result<i32, ItemRuleError> {
    match value {
        ComponentValue::Damage(value) if id == component::DAMAGE => Ok(*value),
        ComponentValue::MaxDamage(value) if id == component::MAX_DAMAGE => Ok(*value),
        ComponentValue::Opaque(value) => {
            let mut reader = BoundedReader::new(value.raw());
            let result = reader.read_var_int().map_err(|_| malformed(id))?;
            if reader.remaining() != 0 {
                return Err(malformed(id));
            }
            Ok(result)
        }
        _ => Err(malformed(id)),
    }
}

/// Resolves `DAMAGE`/`MAX_DAMAGE` additions/removals and the unbreakable unit.
/// Typed signed damage is preserved in storage but clamped only for gameplay.
/// A missing damage or maximum component makes the item non-damageable.
pub fn durability(stack: &ItemStack) -> Result<Option<Durability>, ItemRuleError> {
    let Some(item) = admitted(stack)? else {
        return Ok(None);
    };
    let original_max = builtin_max_damage(item);
    let max = match addition(stack, component::MAX_DAMAGE) {
        Some(value) => Some(integer_component(value, component::MAX_DAMAGE)?),
        None if removed(stack, component::MAX_DAMAGE) => None,
        None => original_max,
    };
    let damage = match addition(stack, component::DAMAGE) {
        Some(value) => Some(integer_component(value, component::DAMAGE)?),
        None if removed(stack, component::DAMAGE) => None,
        None => original_max.map(|_| 0),
    };
    let unbreakable = match addition(stack, component::UNBREAKABLE) {
        Some(ComponentValue::Unbreakable) => true,
        Some(ComponentValue::Opaque(value)) if value.raw().is_empty() => true,
        Some(_) => return Err(malformed(component::UNBREAKABLE)),
        None => false,
    };
    let (Some(max_damage), Some(damage)) = (max, damage) else {
        return Ok(None);
    };
    if max_damage <= 0 {
        return Err(malformed(component::MAX_DAMAGE));
    }
    Ok(Some(Durability {
        damage: damage.clamp(0, max_damage),
        max_damage,
        unbreakable,
    }))
}

/// One checked durability postimage, awaiting an exclusive inventory commit.
#[derive(Debug)]
pub struct DurabilityPlan {
    stack_after: ItemStack,
    broken: bool,
}
impl DurabilityPlan {
    /// Complete conserving item postimage (one item consumed only on break).
    pub fn stack_after(&self) -> &ItemStack {
        &self.stack_after
    }
    /// Whether this operation consumed one broken item.
    pub fn broken(&self) -> bool {
        self.broken
    }
    /// Moves the checked postimage into the authoritative inventory owner.
    pub fn into_stack(self) -> ItemStack {
        self.stack_after
    }
}

/// Plans deterministic positive wear after events/enchanting/creative policy.
///
/// Creative and unbreakable stacks stay byte-identical. Surviving stacks replace
/// only DAMAGE with a typed scalar, preserving all unrelated additions/removals
/// in order. Uninterpreted enchantment overrides refuse this baseline; caller
/// must separately implement their probabilistic unbreaking and event behavior.
pub fn plan_durability_damage(
    stack: &ItemStack,
    amount: u32,
    creative: bool,
) -> Result<DurabilityPlan, ItemRuleError> {
    let state = durability(stack)?;
    let Some(state) = state.filter(|state| !state.unbreakable && !creative && amount != 0) else {
        return Ok(DurabilityPlan {
            stack_after: stack.clone(),
            broken: false,
        });
    };
    refuse_overrides(stack, &[ENCHANTMENTS])?;
    let damage =
        (u64::from(state.damage as u32) + u64::from(amount)).min(state.max_damage as u64) as i32;
    let broken = damage >= state.max_damage;
    let count = stack.count() - u8::from(broken);
    let stack_after = if count == 0 {
        ItemStack::empty()
    } else {
        let mut added = stack.components().added().to_vec();
        if let Some(value) = added
            .iter_mut()
            .find(|value| value.type_id() == component::DAMAGE)
        {
            *value = ComponentValue::Damage(damage);
        } else {
            added.push(ComponentValue::Damage(damage));
        }
        let components = ComponentPatch::new(added, stack.components().removed().to_vec());
        // The input was present and preflighted; preserve its exact id and count.
        match stack.item() {
            Some(item) => {
                ItemStack::try_new(item, count, components).map_err(ItemPersistenceError::from)?
            }
            None => return Err(ItemRuleError::PresentAir),
        }
    };
    persisted_stack_size(&stack_after)?;
    Ok(DurabilityPlan {
        stack_after,
        broken,
    })
}

/// Applies the original unenchanted armor/toughness damage formula.
///
/// Inputs are the final player armor attributes within their original ranges,
/// not an untrusted stack patch. Bypassing damage sources, enchantments and
/// resistance are consumer policy and are not inferred here.
pub fn damage_after_armor(damage: f32, armor: f32, toughness: f32) -> Result<f32, ItemRuleError> {
    if !damage.is_finite()
        || damage < 0.0
        || !armor.is_finite()
        || !(0.0..=30.0).contains(&armor)
        || !toughness.is_finite()
        || !(0.0..=20.0).contains(&toughness)
    {
        return Err(ItemRuleError::InvalidCombatInput);
    }
    let divisor = 2.0 + toughness / 4.0;
    let protection = (armor - damage / divisor).clamp(armor * 0.2, 20.0);
    Ok(damage * (1.0 - protection / 25.0))
}
