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

pub mod component;
pub mod crafting;
pub mod item_id;
pub mod persistence;
pub mod stack;
pub mod transfer;
pub mod untrusted;
pub mod wire;

pub use component::{ComponentPatch, ComponentTypeId, ComponentValue, OpaqueComponent};
pub use crafting::{CraftingError, CraftingGrid, NormalRecipe, NormalRecipes, SingleCraftPlan};
pub use item_id::ItemId;
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
