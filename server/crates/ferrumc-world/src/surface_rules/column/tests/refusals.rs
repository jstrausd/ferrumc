//! C7 — every `build_chunk` refusal kind in the contract §9 order, the buffer
//! `Empty` after each; the two `refused` replay classes (only the typed
//! refusal, never an original value); A1 centre-domain producer refusals and
//! admitted edges; RR-H1 (seed before settings when both differ).
//! These are policy, consistency and defensive tests, not original comparisons.

use ferrumc_registry::biome::BiomeResourceId;

use super::super::super::{ResolverMismatch, SurfaceRuleErrorKind};
use super::*;
use crate::biome_selection::BiomeSourceIdentity;
use crate::density::NoiseChunkErrorKind;
use crate::stored_biome::StoredBiomeErrorKind;
use crate::surface_rules::SurfaceRuleWorkBudget;

/// Publishes the origin group, then runs `input`; returns the refusal and
/// asserts the buffer is `Empty` afterwards.
fn refused_after_publication(
    columns: &SeededSurfaceColumns,
    input: SurfaceChunkInput<'_>,
    budget: u32,
) -> SurfaceColumnError {
    let (group, produced) = produced_group("origin/normal/0");
    let (rules, origin) = bindings(group.settings, group.seed);
    let resolver = produced.resolver();
    let mut buffer = SurfaceChunkBuffer::new().unwrap();
    origin
        .build_chunk(
            SurfaceChunkInput::new(
                rules,
                &resolver,
                temperatures(),
                produced.corners,
                &produced.material,
            ),
            work_budget(SURFACE_CHUNK_WORK),
            &mut buffer,
        )
        .unwrap();
    assert!(buffer.published().is_some());
    let error = columns
        .build_chunk(input, work_budget(budget), &mut buffer)
        .unwrap_err();
    assert_eq!(buffer.published(), None, "{error:?}");
    assert!(buffer.world_surface_heights().is_none());
    assert_eq!(buffer.state(BlockPos::new(0, 0, 0)), None);
    assert_eq!(buffer.fluid_flag(BlockPos::new(0, 0, 0)), None);
    error
}

fn kind_after(
    columns: &SeededSurfaceColumns,
    input: SurfaceChunkInput<'_>,
    budget: u32,
) -> SurfaceColumnErrorKind {
    refused_after_publication(columns, input, budget)
        .kind()
        .clone()
}

/// Rules and column binding for an arbitrary (settings, seed).
fn pair(name: &str, seed: i64) -> &'static (SeededSurfaceRules, SeededSurfaceColumns) {
    bindings(settings(name), seed)
}

/// C7 steps 1–4 in order, with combinations that pin each precedence.
#[test]
fn build_chunk_refusals_follow_the_contract_order_and_leave_the_buffer_empty() {
    use MaterialMismatch as M;
    let (_, origin) = produced_group("origin/normal/0");
    let (_, minus_one) = produced_group("origin/normal/-1");
    let (_, large) = produced_group("origin/large_biomes/0");
    let (_, seam) = produced_group("seam/normal/0/+x");
    let (rules, columns) = pair("minecraft:overworld", 0);
    let origin_resolver = origin.resolver();
    let input = |rules, resolver, material| {
        SurfaceChunkInput::new(rules, resolver, temperatures(), origin.corners, material)
    };
    let r = SURFACE_CHUNK_WORK;

    // 1. Binding: seed, then settings; seed first when both differ.
    let (seed_rules, _) = pair("minecraft:overworld", -1);
    let (large_rules, _) = pair("minecraft:large_biomes", 0);
    let (both_rules, _) = pair("minecraft:large_biomes", 1);
    for (other, mismatch) in [
        (seed_rules, BindingMismatch::Seed),
        (large_rules, BindingMismatch::Settings),
        (both_rules, BindingMismatch::Seed),
    ] {
        let error = refused_after_publication(
            columns,
            input(other, &origin_resolver, &minus_one.material),
            r - 1,
        );
        assert_eq!(
            *error.kind(),
            SurfaceColumnErrorKind::InconsistentBinding(mismatch)
        );
        assert_eq!(error.chunk(), Some(minus_one.material.chunk_pos()));
        assert_eq!(
            error.kind().category(),
            SurfaceColumnRefusalCategory::Consistency
        );
    }

    // 2. Material: seed, settings, geometry, chunk — each before the session.
    let seam_resolver = seam.resolver();
    let truncated = origin
        .material
        .with_states(origin.material.block_states()[..SURFACE_CHUNK_STATES - 1].into());
    for (material, resolver, mismatch) in [
        (&minus_one.material, &origin_resolver, M::Seed),
        (&minus_one.material, &seam_resolver, M::Seed),
        (&large.material, &origin_resolver, M::Settings),
        (&large.material, &seam_resolver, M::Settings),
        (&truncated, &origin_resolver, M::Geometry),
        (&truncated, &seam_resolver, M::Geometry),
        (&seam.material, &origin_resolver, M::Chunk),
    ] {
        assert_eq!(
            kind_after(columns, input(rules, resolver, material), r - 1),
            SurfaceColumnErrorKind::InconsistentMaterial(mismatch)
        );
    }

    // 3. Session admission (GEN-05.9 order) before the driver allowance.
    let minus_one_resolver = minus_one.resolver();
    let large_resolver = large.resolver();
    for (resolver, mismatch) in [
        (&minus_one_resolver, ResolverMismatch::Seed),
        (&large_resolver, ResolverMismatch::Settings),
    ] {
        let kind = kind_after(columns, input(rules, resolver, &origin.material), r - 1);
        assert_eq!(
            kind,
            SurfaceColumnErrorKind::Rules(SurfaceRuleErrorKind::InconsistentResolver(mismatch))
        );
        assert_eq!(kind.category(), SurfaceColumnRefusalCategory::Consistency);
    }

    // 4. The driver allowance last: R − 1 refused with counts, R and R + 1 publish.
    let error = refused_after_publication(
        columns,
        input(rules, &origin_resolver, &origin.material),
        r - 1,
    );
    assert_eq!(*error.kind(), SurfaceColumnErrorKind::WorkBudgetExceeded);
    assert_eq!(
        (error.requested(), error.maximum()),
        (r as usize, (r - 1) as usize)
    );
    assert_eq!(error.kind().category(), SurfaceColumnRefusalCategory::Limit);
    let mut buffer = SurfaceChunkBuffer::new().unwrap();
    for budget in [r, r + 1] {
        let report = columns
            .build_chunk(
                input(rules, &origin_resolver, &origin.material),
                work_budget(budget),
                &mut buffer,
            )
            .unwrap();
        assert!(report.work_used() <= r);
        assert!(buffer.published().is_some());
    }
}

/// RR-H1: seed and settings both differ — the session and `build_chunk` both
/// report `ResolverMismatch::Seed` (GEN-05.9 review mutant R01).
#[test]
fn seed_precedes_settings_when_both_mismatch() {
    let (group, origin) = produced_group("origin/normal/0");
    let resolver = origin.resolver();
    let (rules, columns) = pair("minecraft:large_biomes", 1);
    let session = SurfaceRuleSession::new(
        rules,
        &resolver,
        temperatures(),
        group.corners,
        SurfaceRuleWorkBudget::new(1).unwrap(),
    )
    .unwrap_err();
    assert_eq!(
        *session.kind(),
        SurfaceRuleErrorKind::InconsistentResolver(ResolverMismatch::Seed)
    );
    // Material of the same (large_biomes, 1) passes the driver's own checks.
    let (material, corners) = material(settings("minecraft:large_biomes"), 1, group.center);
    let kind = kind_after(
        columns,
        SurfaceChunkInput::new(rules, &resolver, temperatures(), corners, &material),
        SURFACE_CHUNK_WORK,
    );
    assert_eq!(
        kind,
        SurfaceColumnErrorKind::Rules(SurfaceRuleErrorKind::InconsistentResolver(
            ResolverMismatch::Seed
        ))
    );
}

/// Replay class `refused:InconsistentResolver(Source)` (group
/// `policy/single_biome_surface/0`): only the typed refusal and `Empty`.
#[test]
fn refused_source_group_yields_only_the_typed_refusal() {
    let group = fixture()
        .groups
        .iter()
        .find(|g| g.class == Class::RefusedInconsistentResolverSource)
        .unwrap();
    assert_eq!(group.id, "policy/single_biome_surface/0");
    let plains = BiomeResourceId::from_name("minecraft:plains").unwrap();
    let router = router(group.settings, group.seed);
    let mut source = BiomeSourceSession::fixed(
        plains,
        BiomeWorkBudget::new(MAX_BIOME_SELECTION_WORK).unwrap(),
    );
    let mut arrays = Vec::new();
    for dx in -1..=1 {
        for dz in -1..=1 {
            let chunk = ChunkPos::new(group.center.x() + dx, group.center.z() + dz);
            let mut noise = router
                .fresh_paper_noise_chunk(chunk, noise_budget())
                .unwrap();
            arrays.push(
                source
                    .sample_chunk_biomes(&mut noise, level_geometry())
                    .unwrap(),
            );
        }
    }
    let fixed = StoredBiomeIdentity::new(
        WorldSeed::new(group.seed),
        group.settings,
        BiomeSourceIdentity::Fixed(plains),
        level_geometry(),
    );
    let resolver = resolver_of(fixed, group.center, &arrays);
    let (rules, columns) = bindings(group.settings, group.seed);
    let (_, origin) = produced_group("origin/normal/0");
    let kind = kind_after(
        columns,
        SurfaceChunkInput::new(
            rules,
            &resolver,
            temperatures(),
            origin.corners,
            &origin.material,
        ),
        SURFACE_CHUNK_WORK,
    );
    assert_eq!(
        kind,
        SurfaceColumnErrorKind::Rules(SurfaceRuleErrorKind::InconsistentResolver(
            ResolverMismatch::Source
        ))
    );
    assert_eq!(kind.category(), SurfaceColumnRefusalCategory::Policy);
}

/// Resolver admission with the centre chunk's own genuine GEN-05.5 array:
/// count, centre domain and window pass, then all nine halo domains are
/// checked before presence (`stored_biome.rs` slot order).
fn resolver_kind(center: ChunkPos) -> StoredBiomeErrorKind {
    let router = router(overworld_settings(), 0);
    let mut session = BiomeSourceSession::from_multinoise(BiomeSelectionSession::new(
        biome_tree(),
        BiomeWorkBudget::new(MAX_BIOME_SELECTION_WORK).unwrap(),
    ));
    let mut noise = router
        .fresh_paper_noise_chunk(center, noise_budget())
        .unwrap();
    let own = session
        .sample_chunk_biomes(&mut noise, level_geometry())
        .unwrap();
    SurfaceBiomeResolver::new(
        identity(overworld_settings(), 0),
        center,
        &[&own],
        BiomeZoomSemantics::Paper1_21_8Build60,
        StoredBiomeBuildBudget::new(MAX_STORED_BIOME_BUILD_WORK).unwrap(),
    )
    .unwrap_err()
    .kind()
}

/// A1 — replay class `refused:CentreOutsideDomain` (group
/// `outside/normal/0/+x`, cx 1874999): the halo chunk's GEN-05.5 sampling
/// refuses `ChunkOutOfRange`, the GEN-05.8 resolver `CoordinateDomain`;
/// `build_chunk` is never reached and the buffer stays `Empty`.
#[test]
fn refused_centre_outside_domain_is_refused_by_the_producers() {
    let group = fixture()
        .groups
        .iter()
        .find(|g| g.class == Class::RefusedCentreOutsideDomain)
        .unwrap();
    assert_eq!(
        (group.id.as_str(), group.center),
        ("outside/normal/0/+x", ChunkPos::new(1_874_999, 0))
    );
    let router = router(group.settings, group.seed);
    let halo = router
        .fresh_paper_noise_chunk(ChunkPos::new(1_875_000, 0), noise_budget())
        .err()
        .unwrap();
    assert!(matches!(
        halo.kind(),
        NoiseChunkErrorKind::ChunkOutOfRange {
            axis: "x",
            coordinate: 1_875_000
        }
    ));
    assert_eq!(
        resolver_kind(group.center),
        StoredBiomeErrorKind::CoordinateDomain
    );
    let buffer = SurfaceChunkBuffer::new().unwrap();
    assert_eq!(buffer.published(), None);
}

/// A1 — admitted centre edges `{−1874999, 1874998}` (producer admission
/// only; no oracle group lies there) against the refused `{−1875000, 1874999}`.
#[test]
fn centre_domain_edges_are_admitted_by_the_producers() {
    let router = router(overworld_settings(), 0);
    for (edge, outside, halo) in [
        (-1_874_999, -1_875_000, -1_875_001),
        (1_874_998, 1_874_999, 1_875_000),
    ] {
        for axis in ["x", "z"] {
            let at = |c: i32, other: i32| {
                if axis == "x" {
                    ChunkPos::new(c, other)
                } else {
                    ChunkPos::new(other, c)
                }
            };
            for dx in -1..=1 {
                for dz in -1..=1 {
                    router
                        .fresh_paper_noise_chunk(at(edge + dx, dz), noise_budget())
                        .unwrap();
                }
            }
            // Domain before presence: an admitted centre fails only on presence.
            assert_ne!(
                resolver_kind(at(edge, 0)),
                StoredBiomeErrorKind::CoordinateDomain
            );
            let error = router
                .fresh_paper_noise_chunk(at(halo, 0), noise_budget())
                .err()
                .unwrap();
            assert!(matches!(
                error.kind(),
                NoiseChunkErrorKind::ChunkOutOfRange { axis: a, coordinate } if *a == axis && *coordinate == halo
            ));
            assert_eq!(
                resolver_kind(at(outside, 0)),
                StoredBiomeErrorKind::CoordinateDomain
            );
        }
    }
}

/// C7 step 5, defensive (A5 injection): a state outside the closed alphabet
/// refuses `UnclassifiedState` with its block and leaves the buffer `Empty`.
#[test]
fn unclassified_states_are_defensive_refusals() {
    let (group, origin) = produced_group("origin/normal/0");
    let (rules, _) = pair("minecraft:overworld", 0);
    let resolver = origin.resolver();
    // (a) the binding lacks a state the material contains (stone).
    let mut columns =
        SeededSurfaceColumns::bind(rules, bind_budget(MAX_SURFACE_COLUMN_BIND_WORK)).unwrap();
    let stone =
        BlockStateId::new(ferrumc_registry::block_state::block_default_state("stone").unwrap());
    columns.without_state(stone);
    assert_eq!(columns.alphabet_len(), 34);
    let error = refused_after_publication(
        &columns,
        SurfaceChunkInput::new(
            rules,
            &resolver,
            temperatures(),
            group.corners,
            &origin.material,
        ),
        SURFACE_CHUNK_WORK,
    );
    assert_eq!(*error.kind(), SurfaceColumnErrorKind::UnclassifiedState);
    assert_eq!(
        error.kind().category(),
        SurfaceColumnRefusalCategory::Defensive
    );
    assert!(error.block().is_some() && error.chunk() == Some(group.center));
    // (b) the material carries a state outside the alphabet.
    let (_, columns) = pair("minecraft:overworld", 0);
    let oak =
        BlockStateId::new(ferrumc_registry::block_state::block_default_state("oak_log").unwrap());
    let mut states: Vec<BlockStateId> = origin.material.block_states().to_vec();
    let at = index(5, 0, 7);
    states[at] = oak;
    let foreign = origin.material.with_states(states.into_boxed_slice());
    let error = refused_after_publication(
        columns,
        SurfaceChunkInput::new(rules, &resolver, temperatures(), group.corners, &foreign),
        SURFACE_CHUNK_WORK,
    );
    assert_eq!(*error.kind(), SurfaceColumnErrorKind::UnclassifiedState);
    assert_eq!(error.block(), Some(BlockPos::new(5, 0, 7)));
}

/// A2: the preliminary corners are caller-trusted. The producer corners equal
/// the fixture (`replay` checks every group); foreign corners are not refused
/// — they change the minimum surface and the frozen range of a result that is
/// still published under the material's identity.
#[test]
fn preliminary_corners_are_caller_trusted_input() {
    let (group, produced) = produced_group("biome/deep_frozen_ocean");
    assert_eq!(produced.corners, group.corners);
    let (rules, columns) = bindings(group.settings, group.seed);
    let resolver = produced.resolver();
    let [a, b, c, d] = produced.corners.values();
    let foreign = PreliminarySurfaceCorners::new(a - 40, b - 40, c - 40, d - 40);
    let mut buffer = SurfaceChunkBuffer::new().unwrap();
    let (_, trusted) = drive_recorded(
        columns,
        rules,
        &resolver,
        foreign,
        &produced.material,
        &mut buffer,
    );
    assert_eq!(
        buffer.published(),
        Some((group.center, WorldSeed::new(group.seed), group.settings))
    );
    assert_ne!(
        states_digest(&buffer.states),
        fixture().chunks[group.gid].final_sha
    );
    let rows = &fixture().columns[group.gid];
    let differing = trusted
        .columns
        .iter()
        .zip(rows)
        .filter(|(trace, row)| project(trace)[7] != row.cells[7])
        .count();
    assert!(differing > 0);
}
