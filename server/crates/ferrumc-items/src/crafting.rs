//! Pinned normal crafting recipes and pure, bounded, single-craft plans.
//!
//! Only the original 1.21.8 shaped/shapeless builtins are admitted. Callers
//! explicitly select a recipe; Paper manager ordering, plugin `ExactChoice`,
//! special recipes, inventory/drop placement, GUI clicks, and durable commits
//! belong to their separate consumers. Present-air stacks are refused because
//! the authoritative Rust model preserves them while Minecraft treats them as
//! empty. Normal ingredient matching ignores component patches, and fixed
//! recipe outputs never inherit arbitrary input components.

mod generated;
mod resources;
#[cfg(test)]
mod tests;

use core::mem::size_of;

use crate::persistence::authoritative_encoded_size;
use crate::transfer::copy_at_count;
use crate::{
    ComponentPatch, ComponentTypeId, ComponentValue, ItemId, ItemPersistenceError, ItemStack,
    OpaqueComponent, MAX_COMPONENT_BYTES,
};

/// Maximum total schema-1 bytes admitted in a crafting input grid.
pub const MAX_CRAFTING_GRID_BYTES: usize = 512 * 1024;
/// Maximum logical cloned item/component/NBT bytes admitted in an input grid.
///
/// Counts inline values and cloned lengths, excluding allocator overhead and
/// caller-retained capacities. This is not an RSS guarantee.
pub const MAX_CRAFTING_GRID_CLONE_BYTES: usize = 1024 * 1024;
/// Maximum total schema-1 bytes in the output plus both remainder vectors.
pub const MAX_CRAFT_PLAN_BYTES: usize = 1024 * 1024;
/// Maximum logical bytes cloned for a complete plan, excluding allocator costs.
pub const MAX_CRAFT_PLAN_CLONE_BYTES: usize = 2 * 1024 * 1024;

const MAX_RECIPES: usize = 1024;
const MAX_INGREDIENT_SETS: usize = 1024;
const MAX_SET_MEMBERS: usize = 2048;
const MAX_TOTAL_MEMBERS: usize = 131_072;
const MAX_FIXED_COMPONENTS: usize = 9;
const MAX_CATALOG_BYTES: usize = 1024 * 1024;
const EMPTY_INGREDIENT: u16 = u16::MAX;

/// A classified refusal from crafting admission, catalog loading, or planning.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum CraftingError {
    /// Only vanilla inventory (2×2) and crafting-table (3×3) grids are admitted.
    #[error("unsupported crafting grid side {0}; expected 2 or 3")]
    UnsupportedGridSide(u8),
    /// The supplied slot slice does not fill the declared square grid.
    #[error("crafting grid has {actual} slots; expected {expected}")]
    InvalidGridLength {
        /// Supplied number of slots.
        actual: usize,
        /// Required number of slots.
        expected: usize,
    },
    /// Present-air cannot be silently discarded or normalized at this boundary.
    #[error("present-air stack at crafting slot {slot} is unsupported")]
    PresentAirUnsupported {
        /// Original grid position.
        slot: usize,
    },
    /// The generated closed catalog violated a structural or registry pin.
    #[error("invalid normal crafting catalog: {0}")]
    InvalidCatalog(&'static str),
    /// An aggregate encoded or logical clone allowance was exceeded.
    #[error("crafting resource {resource} needs {actual} bytes; maximum {max}")]
    ResourceLimit {
        /// The specific aggregate allowance.
        resource: &'static str,
        /// Accounted size.
        actual: usize,
        /// Maximum permitted size.
        max: usize,
    },
    /// Checked accounting overflowed; no partial plan is returned.
    #[error("crafting resource accounting overflow")]
    ResourceOverflow,
    /// An authoritative stack failed the existing lossless persistence preflight.
    #[error(transparent)]
    InvalidStack(#[from] ItemPersistenceError),
}

/// A borrowed, fully admitted 2×2 or 3×3 crafting grid.
///
/// Construction allocates and clones nothing. Every stack is validated without
/// creative normalization, then aggregate encoded and logical clone budgets
/// are enforced. Original slot positions remain stable after trimming.
#[derive(Debug, Clone, Copy)]
pub struct CraftingGrid<'a> {
    side: u8,
    stacks: &'a [ItemStack],
    occupied: u16,
    left: u8,
    top: u8,
    width: u8,
    height: u8,
}

impl<'a> CraftingGrid<'a> {
    /// Validates a complete grid without allocating or mutating caller input.
    ///
    /// # Errors
    /// Rejects unsupported sides, wrong slice lengths, invalid lossless stacks,
    /// present-air, or an aggregate encoded/logical clone budget violation.
    pub fn new(side: u8, stacks: &'a [ItemStack]) -> Result<Self, CraftingError> {
        if !matches!(side, 2 | 3) {
            return Err(CraftingError::UnsupportedGridSide(side));
        }
        let expected = usize::from(side) * usize::from(side);
        if stacks.len() != expected {
            return Err(CraftingError::InvalidGridLength {
                actual: stacks.len(),
                expected,
            });
        }
        let mut encoded = 0;
        let mut cloned = 0;
        let mut occupied = 0;
        let (mut left, mut top, mut right, mut bottom) = (side, side, 0, 0);
        for (position, stack) in stacks.iter().enumerate() {
            account_stack(stack, &mut encoded, &mut cloned)?;
            if let Some(item) = stack.item() {
                if item.is_air() {
                    return Err(CraftingError::PresentAirUnsupported { slot: position });
                }
                occupied |= 1 << position;
                let x = (position % usize::from(side)) as u8;
                let y = (position / usize::from(side)) as u8;
                left = left.min(x);
                top = top.min(y);
                right = right.max(x);
                bottom = bottom.max(y);
            }
        }
        limit("grid encoded", encoded, MAX_CRAFTING_GRID_BYTES)?;
        limit("grid clone", cloned, MAX_CRAFTING_GRID_CLONE_BYTES)?;
        let (width, height) = if occupied == 0 {
            (0, 0)
        } else {
            (right - left + 1, bottom - top + 1)
        };
        Ok(Self {
            side,
            stacks,
            occupied,
            left,
            top,
            width,
            height,
        })
    }

    /// The original square grid side (2 or 3).
    #[must_use]
    pub fn side(&self) -> u8 {
        self.side
    }

    /// The original caller-owned slots, including holes and outer empty space.
    #[must_use]
    pub fn stacks(&self) -> &'a [ItemStack] {
        self.stacks
    }
}

#[derive(Debug)]
struct RawOutput {
    item: &'static str,
    count: u8,
    added: &'static [(i32, &'static [u8])],
    removed: &'static [i32],
}

#[derive(Debug)]
struct RawRecipe {
    name: &'static str,
    shaped: bool,
    width: u8,
    height: u8,
    slots: &'static [u16],
    result: RawOutput,
}

/// The immutable normal builtin catalog pinned to Minecraft 1.21.8.
///
/// Construction checks all descriptors and registry pins before allocating
/// recipe outputs. Shared ingredient sets and resource names are static.
#[derive(Debug)]
pub struct NormalRecipes {
    recipes: Vec<NormalRecipe>,
}

impl NormalRecipes {
    /// Materializes all 673 shaped and 295 shapeless original builtin recipes.
    ///
    /// # Errors
    /// Rejects malformed generated descriptors, resource limits, registry pin
    /// disagreement, or a fixed result failing lossless stack validation.
    pub fn builtin() -> Result<Self, CraftingError> {
        validate_catalog(
            generated::INGREDIENTS,
            generated::RECIPES,
            generated::REMAINDERS,
            generated::ITEM_PINS,
        )?;
        let mut recipes = Vec::with_capacity(generated::RECIPES.len());
        for raw in generated::RECIPES {
            let mut added = Vec::with_capacity(raw.result.added.len());
            for (id, bytes) in raw.result.added {
                let value = OpaqueComponent::new(*id, bytes.to_vec())
                    .ok_or(CraftingError::InvalidCatalog("fixed component"))?;
                added.push(ComponentValue::Opaque(value));
            }
            let removed = raw
                .result
                .removed
                .iter()
                .map(|id| ComponentTypeId::new(*id))
                .collect();
            let item = canonical_item(raw.result.item)?;
            let result =
                ItemStack::try_new(item, raw.result.count, ComponentPatch::new(added, removed))
                    .map_err(ItemPersistenceError::from)?;
            authoritative_encoded_size(&result)?;
            recipes.push(NormalRecipe {
                raw,
                ingredients: generated::INGREDIENTS,
                remainders: generated::REMAINDERS,
                output: result,
            });
        }
        Ok(Self { recipes })
    }

    /// Iterates all recipes in resource-name order; this is not manager priority.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &NormalRecipe> {
        self.recipes.iter()
    }

    /// Looks up an explicitly chosen canonical resource name.
    ///
    /// Recipe selection priority, last-recipe caches and plugin registration
    /// remain responsibilities of the future recipe-manager consumer.
    #[must_use]
    pub fn recipe(&self, name: &str) -> Option<&NormalRecipe> {
        self.recipes
            .binary_search_by_key(&name, |recipe| recipe.name())
            .ok()
            .map(|index| &self.recipes[index])
    }
}

/// One explicitly selected shaped or shapeless original builtin recipe.
#[derive(Debug)]
pub struct NormalRecipe {
    raw: &'static RawRecipe,
    ingredients: &'static [&'static [i32]],
    remainders: &'static [(&'static str, &'static str)],
    output: ItemStack,
}

impl NormalRecipe {
    /// The canonical original recipe resource name.
    #[must_use]
    pub fn name(&self) -> &'static str {
        self.raw.name
    }

    /// Whether this is a shaped recipe; otherwise it is normal shapeless.
    #[must_use]
    pub fn is_shaped(&self) -> bool {
        self.raw.shaped
    }

    /// The fixed original result; callers cannot mutate the catalog.
    #[must_use]
    pub fn output(&self) -> &ItemStack {
        &self.output
    }

    /// Checks an admitted grid, with vanilla trim/mirror or complete assignment.
    ///
    /// Counts occupied positions rather than stack item totals. Component
    /// patches never influence the item/tag membership of these normal recipes.
    #[must_use]
    pub fn matches(&self, grid: &CraftingGrid<'_>) -> bool {
        let ingredient_count = self
            .raw
            .slots
            .iter()
            .filter(|id| **id != EMPTY_INGREDIENT)
            .count();
        if grid.occupied.count_ones() as usize != ingredient_count {
            return false;
        }
        if self.raw.shaped {
            if grid.width != self.raw.width || grid.height != self.raw.height {
                return false;
            }
            self.matches_shaped(grid, false) || self.matches_shaped(grid, true)
        } else {
            let mut owners = [usize::MAX; 9];
            for ingredient in 0..self.raw.slots.len() {
                let mut visited = 0;
                if !self.augment(ingredient, grid, &mut owners, &mut visited) {
                    return false;
                }
            }
            true
        }
    }

    fn accepts(&self, set: u16, item: Option<ItemId>) -> bool {
        match (set, item) {
            (EMPTY_INGREDIENT, None) => true,
            (EMPTY_INGREDIENT, Some(_)) | (_, None) => false,
            (set, Some(item)) => self.ingredients[usize::from(set)]
                .binary_search(&item.id())
                .is_ok(),
        }
    }

    fn matches_shaped(&self, grid: &CraftingGrid<'_>, mirrored: bool) -> bool {
        for y in 0..usize::from(self.raw.height) {
            for x in 0..usize::from(self.raw.width) {
                let ingredient_x = if mirrored {
                    usize::from(self.raw.width) - 1 - x
                } else {
                    x
                };
                let set = self.raw.slots[y * usize::from(self.raw.width) + ingredient_x];
                let position = (usize::from(grid.top) + y) * usize::from(grid.side)
                    + usize::from(grid.left)
                    + x;
                if !self.accepts(set, grid.stacks[position].item()) {
                    return false;
                }
            }
        }
        true
    }

    fn augment(
        &self,
        ingredient: usize,
        grid: &CraftingGrid<'_>,
        owners: &mut [usize; 9],
        visited: &mut u16,
    ) -> bool {
        for (position, stack) in grid.stacks.iter().enumerate() {
            if *visited & (1 << position) != 0
                || !self.accepts(self.raw.slots[ingredient], stack.item())
            {
                continue;
            }
            *visited |= 1 << position;
            if owners[position] == usize::MAX
                || self.augment(owners[position], grid, owners, visited)
            {
                owners[position] = ingredient;
                return true;
            }
        }
        false
    }

    /// Plans exactly one craft, keeping grid leftovers and crafting rests apart.
    ///
    /// No match returns `Ok(None)`. A match consumes one item from every
    /// occupied position, preserves remaining patch representation, and copies
    /// the fixed recipe result. Sim must place/merge the separate crafting
    /// rests under its authoritative inventory/drop and durability rules.
    ///
    /// # Errors
    /// Refuses invalid postimages or aggregate plan resource limits before the
    /// first clone. No input is changed and no partial plan is returned.
    pub fn plan_one(
        &self,
        grid: &CraftingGrid<'_>,
    ) -> Result<Option<SingleCraftPlan>, CraftingError> {
        if !self.matches(grid) {
            return Ok(None);
        }
        let mut encoded = 0;
        let mut cloned = size_of::<SingleCraftPlan>();
        account_stack(&self.output, &mut encoded, &mut cloned)?;
        let mut rests: [Option<ItemId>; 9] = [None; 9];
        for (position, stack) in grid.stacks.iter().enumerate() {
            // Only surviving postimages will clone patches; drained slots are
            // canonical empty. Validate the original again before any copying.
            authoritative_encoded_size(stack)?;
            if stack.count() > 1 {
                account_stack(stack, &mut encoded, &mut cloned)?;
            } else {
                account_stack(&ItemStack::empty(), &mut encoded, &mut cloned)?;
            }
            if let Some(item) = stack.item() {
                if let Ok(index) = self
                    .remainders
                    .binary_search_by_key(&item.name(), |pair| pair.0)
                {
                    rests[position] = Some(canonical_item(self.remainders[index].1)?);
                }
            }
            let remainder = match rests[position] {
                Some(item) => ItemStack::try_new(item, 1, ComponentPatch::empty())
                    .map_err(ItemPersistenceError::from)?,
                None => ItemStack::empty(),
            };
            account_stack(&remainder, &mut encoded, &mut cloned)?;
        }
        limit("plan encoded", encoded, MAX_CRAFT_PLAN_BYTES)?;
        limit("plan clone", cloned, MAX_CRAFT_PLAN_CLONE_BYTES)?;
        let output = copy_at_count(&self.output, self.output.count())?;
        let mut grid_after = Vec::with_capacity(grid.stacks.len());
        let mut crafting_remainders = Vec::with_capacity(grid.stacks.len());
        for (position, stack) in grid.stacks.iter().enumerate() {
            grid_after.push(copy_at_count(stack, stack.count().saturating_sub(1))?);
            crafting_remainders.push(match rests[position] {
                Some(item) => ItemStack::try_new(item, 1, ComponentPatch::empty())
                    .map_err(ItemPersistenceError::from)?,
                None => ItemStack::empty(),
            });
        }
        Ok(Some(SingleCraftPlan {
            recipe: self.name(),
            side: grid.side,
            output,
            grid_after,
            crafting_remainders,
            consumed_mask: grid.occupied,
        }))
    }
}

/// A complete owned snapshot for one successful normal craft.
///
/// This is a pure plan with no revision/CAS, durable commit, event, inventory
/// merge, or drop authority. Vectors use the original untrimmed grid positions.
#[derive(Debug)]
pub struct SingleCraftPlan {
    recipe: &'static str,
    side: u8,
    output: ItemStack,
    grid_after: Vec<ItemStack>,
    crafting_remainders: Vec<ItemStack>,
    consumed_mask: u16,
}

impl SingleCraftPlan {
    /// The explicitly selected recipe resource name.
    #[must_use]
    pub fn recipe(&self) -> &'static str {
        self.recipe
    }
    /// The original square grid side (2 or 3).
    #[must_use]
    pub fn side(&self) -> u8 {
        self.side
    }
    /// The fixed assembled recipe result.
    #[must_use]
    pub fn output(&self) -> &ItemStack {
        &self.output
    }
    /// Original-position grid postimages after consuming one item per ingredient.
    #[must_use]
    pub fn grid_after(&self) -> &[ItemStack] {
        &self.grid_after
    }
    /// Separate original-position crafting rests, awaiting authoritative placement.
    #[must_use]
    pub fn crafting_remainders(&self) -> &[ItemStack] {
        &self.crafting_remainders
    }
    /// Bit `i` is set exactly when original slot `i` consumed one item.
    #[must_use]
    pub fn consumed_mask(&self) -> u16 {
        self.consumed_mask
    }
    /// Moves the result and both original-position vectors into the consumer.
    #[must_use]
    pub fn into_parts(self) -> (ItemStack, Vec<ItemStack>, Vec<ItemStack>) {
        (self.output, self.grid_after, self.crafting_remainders)
    }
}

fn checked_add(total: &mut usize, size: usize) -> Result<(), CraftingError> {
    *total = total
        .checked_add(size)
        .ok_or(CraftingError::ResourceOverflow)?;
    Ok(())
}

fn account_stack(
    stack: &ItemStack,
    encoded: &mut usize,
    cloned: &mut usize,
) -> Result<(), CraftingError> {
    checked_add(encoded, authoritative_encoded_size(stack)?)?;
    checked_add(cloned, resources::clone_bytes(stack)?)
}

fn limit(resource: &'static str, actual: usize, max: usize) -> Result<(), CraftingError> {
    if actual > max {
        return Err(CraftingError::ResourceLimit {
            resource,
            actual,
            max,
        });
    }
    Ok(())
}

fn canonical_item(name: &'static str) -> Result<ItemId, CraftingError> {
    // Existing public name lookup canonicalizes through an allocated String.
    // These fixed generated names can instead use the validated static pin map.
    let index = generated::ITEM_PINS
        .binary_search_by_key(&name, |pin| pin.0)
        .map_err(|_| CraftingError::InvalidCatalog("unknown item"))?;
    let item = ItemId::new(generated::ITEM_PINS[index].1)
        .ok_or(CraftingError::InvalidCatalog("unknown item"))?;
    if item.name() != name || item.is_air() {
        return Err(CraftingError::InvalidCatalog("noncanonical or air item"));
    }
    Ok(item)
}

fn validate_catalog(
    sets: &[&[i32]],
    recipes: &[RawRecipe],
    remainders: &[(&'static str, &'static str)],
    pins: &[(&str, i32, u8)],
) -> Result<(), CraftingError> {
    if recipes.is_empty()
        || recipes.len() > MAX_RECIPES
        || sets.is_empty()
        || sets.len() > MAX_INGREDIENT_SETS
        || remainders.len() > MAX_SET_MEMBERS
        || pins.len() > MAX_SET_MEMBERS
    {
        return Err(CraftingError::InvalidCatalog("catalog count limits"));
    }
    let mut memberships = 0;
    for set in sets {
        if set.is_empty()
            || set.len() > MAX_SET_MEMBERS
            || !set.windows(2).all(|pair| pair[0] < pair[1])
        {
            return Err(CraftingError::InvalidCatalog("ingredient set"));
        }
        checked_add(&mut memberships, set.len())?;
        for id in *set {
            if ItemId::new(*id).is_none_or(ItemId::is_air) {
                return Err(CraftingError::InvalidCatalog("ingredient item"));
            }
        }
    }
    if memberships > MAX_TOTAL_MEMBERS {
        return Err(CraftingError::InvalidCatalog("membership total"));
    }
    for (name, id, max) in pins {
        let item = ItemId::new(*id).ok_or(CraftingError::InvalidCatalog("item registry pin"))?;
        if item.name() != *name || item.max_stack() != *max {
            return Err(CraftingError::InvalidCatalog("item registry pin"));
        }
    }
    if !recipes.windows(2).all(|pair| pair[0].name < pair[1].name)
        || !remainders.windows(2).all(|pair| pair[0].0 < pair[1].0)
    {
        return Err(CraftingError::InvalidCatalog(
            "duplicate or unordered resources",
        ));
    }
    let mut catalog_bytes = recipes
        .len()
        .checked_mul(size_of::<NormalRecipe>())
        .ok_or(CraftingError::ResourceOverflow)?;
    for raw in recipes {
        validate_recipe(raw, sets.len(), &mut catalog_bytes)?;
    }
    for (source, result) in remainders {
        canonical_item(source)?;
        canonical_item(result)?;
    }
    limit("catalog clone", catalog_bytes, MAX_CATALOG_BYTES)
}

fn validate_recipe(
    raw: &RawRecipe,
    set_count: usize,
    catalog_bytes: &mut usize,
) -> Result<(), CraftingError> {
    if !valid_resource(raw.name)
        || raw.slots.is_empty()
        || raw.slots.len() > 9
        || raw.slots.iter().all(|set| *set == EMPTY_INGREDIENT)
    {
        return Err(CraftingError::InvalidCatalog("recipe descriptor"));
    }
    if raw.shaped {
        if !(1..=3).contains(&raw.width)
            || !(1..=3).contains(&raw.height)
            || raw.slots.len() != usize::from(raw.width) * usize::from(raw.height)
        {
            return Err(CraftingError::InvalidCatalog("shaped dimensions"));
        }
        let width = usize::from(raw.width);
        if raw.slots[..width].iter().all(|id| *id == EMPTY_INGREDIENT)
            || raw.slots[raw.slots.len() - width..]
                .iter()
                .all(|id| *id == EMPTY_INGREDIENT)
            || (0..usize::from(raw.height)).all(|y| raw.slots[y * width] == EMPTY_INGREDIENT)
            || (0..usize::from(raw.height))
                .all(|y| raw.slots[y * width + width - 1] == EMPTY_INGREDIENT)
        {
            return Err(CraftingError::InvalidCatalog("untrimmed pattern"));
        }
    } else if raw.width != 0 || raw.height != 0 || raw.slots.contains(&EMPTY_INGREDIENT) {
        return Err(CraftingError::InvalidCatalog("shapeless descriptor"));
    }
    if raw
        .slots
        .iter()
        .any(|set| *set != EMPTY_INGREDIENT && usize::from(*set) >= set_count)
    {
        return Err(CraftingError::InvalidCatalog("ingredient index"));
    }
    let result = &raw.result;
    let item = canonical_item(result.item)?;
    if result.count == 0
        || result.count > item.max_stack()
        || result.added.len() + result.removed.len() > MAX_FIXED_COMPONENTS
    {
        return Err(CraftingError::InvalidCatalog("fixed output"));
    }
    let mut seen = [false; 96];
    checked_add(
        catalog_bytes,
        result.added.len() * size_of::<ComponentValue>()
            + result.removed.len() * size_of::<ComponentTypeId>(),
    )?;
    for (id, bytes) in result.added {
        let type_id = ComponentTypeId::new(*id);
        if !type_id.is_in_range() || type_id.is_dangerous() || bytes.len() > MAX_COMPONENT_BYTES {
            return Err(CraftingError::InvalidCatalog("fixed component"));
        }
        if core::mem::replace(&mut seen[*id as usize], true) {
            return Err(CraftingError::InvalidCatalog("duplicate fixed component"));
        }
        checked_add(catalog_bytes, bytes.len())?;
    }
    for id in result.removed {
        if !ComponentTypeId::new(*id).is_in_range()
            || core::mem::replace(&mut seen[*id as usize], true)
        {
            return Err(CraftingError::InvalidCatalog("fixed removal"));
        }
    }
    Ok(())
}

fn valid_resource(name: &str) -> bool {
    name.len() <= 256
        && name.strip_prefix("minecraft:").is_some_and(|path| {
            !path.is_empty()
                && path.bytes().all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || matches!(byte, b'_' | b'/' | b'.')
                })
        })
}
