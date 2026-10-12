//! C6 — work constants, allowance domains, the bind census, the derived
//! closed alphabet (A5) and the retained-byte caps at N−1/N/N+1.
//! Limit and policy tests; no original row is read as an expectation.

use std::mem::size_of;

use ferrumc_registry::block_state::{block_default_state, block_metadata};

use super::super::bind::StateClass;
use super::*;

/// Pinned measured retained bytes (x86-64, this toolchain).
const BINDING_RETAINED_BYTES: usize = 10_240 + size_of::<SeededSurfaceColumns>();
const BUFFER_RETAINED_BYTES: usize = 393_216 + 12_288 + size_of::<SurfaceChunkBuffer>();

#[test]
fn work_constants_follow_their_formulas() {
    assert_eq!(SURFACE_COLUMN_BIOME_WORK, 1);
    assert_eq!(SURFACE_COLUMN_ERODED_WORK, 1155);
    assert_eq!(SURFACE_COLUMN_LOOP_WORK, 1538);
    assert_eq!(SURFACE_COLUMN_FROZEN_WORK, 1159);
    assert_eq!(SURFACE_COLUMN_WORK, 1 + 1159 + 1538);
    assert_eq!(SURFACE_COLUMN_WORK, 2698);
    assert_eq!(SURFACE_CHUNK_FIXED_WORK, 198_144);
    assert_eq!(SURFACE_CHUNK_WORK, 198_144 + 256 * 2698);
    assert_eq!(SURFACE_CHUNK_WORK, 888_832);
    const { assert!(SURFACE_CHUNK_WORK < MAX_SURFACE_COLUMN_WORK) };
    assert_eq!(SURFACE_CHUNK_STATES, 98_304);
    assert_eq!(SURFACE_CHUNK_FLUID_WORDS, 1_536);
}

#[test]
fn allowance_domains_are_checked_at_both_ends() {
    for (units, admitted) in [(0, false), (1, true), (65_536, true), (65_537, false)] {
        let result = SurfaceColumnBindBudget::new(units);
        assert_eq!(result.is_ok(), admitted, "{units}");
        if let Err(error) = result {
            assert_eq!(*error.kind(), SurfaceColumnErrorKind::InvalidBudget);
            assert_eq!(
                (error.requested(), error.maximum()),
                (units as usize, 65_536)
            );
        }
    }
    for (units, admitted) in [(0, false), (1, true), (1_048_576, true), (1_048_577, false)] {
        let result = SurfaceColumnWorkBudget::new(units);
        assert_eq!(result.is_ok(), admitted, "{units}");
        if let Err(error) = result {
            assert_eq!(*error.kind(), SurfaceColumnErrorKind::InvalidBudget);
            assert_eq!(
                (error.requested(), error.maximum()),
                (units as usize, 1_048_576)
            );
            assert_eq!(error.kind().category(), SurfaceColumnRefusalCategory::Limit);
        }
    }
}

/// A5: the closed alphabet is derived at bind time and has 35 states for all
/// three admitted settings; its classes follow the block names.
#[test]
fn derived_alphabet_has_35_states_for_every_admitted_setting() {
    let water = block_metadata("water").unwrap();
    let lava = block_metadata("lava").unwrap();
    let stone = block_default_state("stone").unwrap();
    for name in [
        "minecraft:overworld",
        "minecraft:large_biomes",
        "minecraft:amplified",
    ] {
        let (rules, columns) = bindings(settings(name), 0);
        assert_eq!(columns.alphabet_len(), 35, "{name}");
        assert_eq!(rules.template().states.len(), 24, "{name}");
        let alphabet: Vec<u32> = columns
            .alphabet
            .states()
            .map(BlockStateId::as_u32)
            .collect();
        let mut sorted = alphabet.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted, alphabet);
        // Every observed original state, and the three unobserved members.
        for raw in &fixture().raw {
            assert!(alphabet.contains(raw), "{name} {raw}");
        }
        for extra in ["calcite", "red_sandstone", "raw_copper_block"] {
            assert!(
                alphabet.contains(&block_default_state(extra).unwrap()),
                "{extra}"
            );
        }
        for raw in alphabet {
            let class = columns.alphabet.classify(BlockStateId::new(raw)).unwrap();
            let block = ferrumc_registry::block_state::state_id_to_block_name(raw).unwrap();
            assert_eq!(
                class.air(),
                matches!(block, "air" | "cave_air" | "void_air")
            );
            let in_water = (water.min_state..=water.max_state).contains(&raw);
            let in_lava = (lava.min_state..=lava.max_state).contains(&raw);
            assert_eq!(class.fluid(), in_water || in_lava, "{block}");
            assert_eq!(class.water_block(), in_water, "{block}");
            assert_eq!(class.default_block(), block == "stone", "{block}");
            assert_eq!(class.default_state(), raw == stone, "{block}");
        }
        assert!(StateClass::VOID_AIR.air());
        assert!(!StateClass::VOID_AIR.fluid() && !StateClass::VOID_AIR.water_block());
    }
}

#[test]
fn bind_census_is_admitted_from_n() {
    let (rules, columns) = bindings(overworld_settings(), 0);
    assert_eq!(SURFACE_COLUMN_NOISE_BIND_WORK, 8320);
    assert_eq!(columns.construction_work(), 8320 + 35);
    for (units, admitted) in [(8354, false), (8355, true), (8356, true)] {
        let result = SeededSurfaceColumns::bind(rules, bind_budget(units));
        assert_eq!(result.is_ok(), admitted, "{units}");
        if let Err(error) = result {
            assert_eq!(*error.kind(), SurfaceColumnErrorKind::WorkBudgetExceeded);
            assert_eq!((error.requested(), error.maximum()), (8355, 8354));
        }
    }
    let bound = SeededSurfaceColumns::bind(rules, bind_budget(8355)).unwrap();
    assert_eq!(
        (bound.seed(), bound.resource()),
        (rules.seed(), rules.template().resource())
    );
}

#[test]
fn retained_byte_caps_are_enforced_at_n() {
    let (rules, columns) = bindings(overworld_settings(), 0);
    let retained = columns.retained_bytes();
    assert_eq!(retained, BINDING_RETAINED_BYTES);
    assert!(retained <= MAX_SURFACE_COLUMN_BINDING_BYTES);
    let budget = bind_budget(MAX_SURFACE_COLUMN_BIND_WORK);
    for (cap, admitted) in [
        (retained - 1, false),
        (retained, true),
        (retained + 1, true),
    ] {
        let result =
            SeededSurfaceColumns::bind_with(rules, budget, cap, MAX_SURFACE_COLUMN_ALPHABET);
        assert_eq!(result.is_ok(), admitted, "{cap}");
        if let Err(error) = result {
            assert_eq!(*error.kind(), SurfaceColumnErrorKind::NativeLimit);
            assert_eq!(error.limit(), Some(SurfaceColumnLimit::BindingBytes));
            assert_eq!((error.requested(), error.maximum()), (retained, cap));
        }
    }
    for (cap, admitted) in [(34, false), (35, true), (36, true)] {
        let result =
            SeededSurfaceColumns::bind_with(rules, budget, MAX_SURFACE_COLUMN_BINDING_BYTES, cap);
        assert_eq!(result.is_ok(), admitted, "{cap}");
        if let Err(error) = result {
            assert_eq!(error.limit(), Some(SurfaceColumnLimit::AlphabetStates));
            assert_eq!((error.requested(), error.maximum()), (35, 34));
        }
    }
    let buffer = SurfaceChunkBuffer::new().unwrap();
    let retained = buffer.retained_bytes();
    assert_eq!(retained, BUFFER_RETAINED_BYTES);
    assert!(retained <= MAX_SURFACE_CHUNK_BUFFER_BYTES);
    for (cap, admitted) in [
        (retained - 1, false),
        (retained, true),
        (retained + 1, true),
    ] {
        let result = SurfaceChunkBuffer::new_with(cap);
        assert_eq!(result.is_ok(), admitted, "{cap}");
        if let Err(error) = result {
            assert_eq!(error.limit(), Some(SurfaceColumnLimit::BufferBytes));
            assert_eq!((error.requested(), error.maximum()), (retained, cap));
        }
    }
    println!(
        "GEN0510_RETAINED binding {} buffer {} binding_value {} buffer_value {}",
        columns.retained_bytes(),
        retained,
        size_of::<SeededSurfaceColumns>(),
        size_of::<SurfaceChunkBuffer>()
    );
}
