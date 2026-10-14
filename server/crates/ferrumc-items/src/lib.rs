#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

//! See the crate README for the trusted/untrusted slot distinction and the
//! hostile-input normalization rules. The module layout:
//!
//! * [`item_id`] — the registry-validated [`ItemId`].
//! * [`component`] — the data-component model ([`ComponentValue`],
//!   [`ComponentPatch`], [`ComponentTypeId`]) and the component type-id constants.
//! * [`stack`] — the canonical [`ItemStack`] and its trusted slot encoder.
//! * [`untrusted`] — the [`UntrustedItemStack`] wire form, its decoder, and
//!   `into_validated`.
//! * [`wire`] — shared wire bounds and the `SetContainerContent` payload builder.
//! * [`persistence`] — versioned, bounded, lossless storage of modelled stacks.
//! * [`transfer`] — checked pure split and non-swapping transfer plans.
//! * [`crafting`] — pinned normal recipes and conservative single-craft plans.
//! * [`metadata`] — bounded trusted `DATA_ITEM` metadata with exact size admission.

pub mod component;
pub mod container;
pub mod crafting;
pub mod item_id;
pub mod metadata;
pub mod persistence;
pub mod rules;
pub mod smelting;
pub mod stack;
pub mod transfer;
pub mod untrusted;
pub mod wire;

pub use component::{ComponentPatch, ComponentTypeId, ComponentValue, OpaqueComponent};
pub use container::{
    exchange_slot, plan_chest_drag, plan_chest_inventory_click, stack_capacity,
    transfer_into_slots, ChestDragPlan, ContainerTransferError,
};
pub use crafting::{CraftingError, CraftingGrid, NormalRecipe, NormalRecipes, SingleCraftPlan};
pub use item_id::ItemId;
pub use metadata::{
    encode_item_entity_metadata, ItemEntityMetadata, ItemMetadataError,
    MAX_ITEM_ENTITY_METADATA_BYTES,
};
pub use persistence::{
    decode_persisted_stack, encode_persisted_stack, persisted_stack_size, ItemPersistenceError,
    MAX_PERSISTED_STACK_BYTES,
};
pub use stack::{left_click_exchange, ItemStack};
pub use transfer::{
    plan_split, plan_transfer, same_stack_representation, StackTransferError, StackTransferPlan,
};
pub use untrusted::{ItemValidationError, UntrustedItemStack};
pub use wire::{
    encode_container_content_payload, MAX_COMPONENTS, MAX_COMPONENTS_TOTAL_BYTES,
    MAX_COMPONENT_BYTES, MAX_WINDOW_SLOTS,
};

pub use rules::{
    armor_properties, builtin_armor, builtin_food, builtin_max_damage, builtin_tool,
    damage_after_armor, durability, food_properties, plan_durability_damage, tool_properties,
    ArmorProperties, ArmorSlot, Durability, DurabilityPlan, FoodEffect, FoodEffectKind,
    FoodProperties, ItemRuleError, ToolProperties,
};
pub use smelting::{
    fuel_properties, plan_furnace_tick, smelting_recipe, validate_furnace_parts, FuelProperties,
    FurnaceTickPlan, SmeltingError, SmeltingRecipe, FURNACE_FRAMING_BYTES, MAX_FURNACE_BYTES,
};

pub mod furnace_container;
pub use furnace_container::{plan_furnace_click, FurnaceClickError, FurnaceClickPlan};
