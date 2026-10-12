//! `FerrumC` policy refusals (P1–P4, P6) and input checks, separate from
//! original behaviour. Each policy kind is distinguishable (A8).

use super::*;

pub(super) fn origin() -> (&'static Group, &'static Generated) {
    let generated = &generated()[0];
    let group = &fixture().groups[generated.gid];
    assert_eq!(group.id, "origin/normal/0");
    (group, generated)
}

/// Real GEN-05.5 arrays of the origin group's nine chunks for another source or geometry.
fn arrays(source: BiomeSourceIdentity, geometry: BiomeQuartGeometry) -> Vec<ChunkBiomeSamples> {
    let budget = || BiomeWorkBudget::new(MAX_BIOME_SELECTION_WORK).unwrap();
    let mut session = match source {
        BiomeSourceIdentity::MultiNoise(preset) => {
            BiomeSourceSession::from_multinoise(BiomeSelectionSession::new(
                Arc::new(
                    BiomeTree::from_builtin(
                        preset,
                        BiomeBuildBudget::new(MAX_BIOME_BUILD_WORK).unwrap(),
                    )
                    .unwrap(),
                ),
                budget(),
            ))
        }
        BiomeSourceIdentity::Fixed(biome) => BiomeSourceSession::fixed(biome, budget()),
        BiomeSourceIdentity::End => BiomeSourceSession::the_end(budget()).unwrap(),
    };
    let router = RouterTemplate::compile_builtin(settings("minecraft:overworld"))
        .unwrap()
        .bind_seed(WorldSeed::new(0))
        .unwrap();
    let mut out = Vec::new();
    for dx in -1..=1 {
        for dz in -1..=1 {
            let mut noise = router
                .fresh_paper_noise_chunk(
                    ChunkPos::new(dx, dz),
                    NoiseChunkWorkBudget::new(MAX_NOISE_CHUNK_WORK).unwrap(),
                )
                .unwrap();
            out.push(session.sample_chunk_biomes(&mut noise, geometry).unwrap());
        }
    }
    out
}

fn identity(source: BiomeSourceIdentity, geometry: BiomeQuartGeometry) -> StoredBiomeIdentity {
    StoredBiomeIdentity::new(
        WorldSeed::new(0),
        settings("minecraft:overworld"),
        source,
        geometry,
    )
}

fn session_error(
    rules: &SeededSurfaceRules,
    resolver: &SurfaceBiomeResolver<'_>,
) -> SurfaceRuleErrorKind {
    SurfaceRuleSession::new(
        rules,
        resolver,
        temperatures(),
        PreliminarySurfaceCorners::new(0, 0, 0, 0),
        work_budget(1),
    )
    .unwrap_err()
    .kind()
    .clone()
}

/// P1/P2: typed policy refusals, profile before flat bedrock, both before work.
#[test]
fn numeric_profile_and_flat_bedrock_are_refused_before_any_work() {
    use PaperFlatBedrock::{Disabled, Enabled};
    use SurfaceNumericProfile::{PaperBuild60, Vanilla};
    let template = overworld();
    let refuse = |profile, flat, units| {
        SeededSurfaceRules::bind(
            Arc::clone(&template),
            WorldSeed::new(0),
            profile,
            flat,
            bind_budget(units),
        )
        .unwrap_err()
        .kind()
        .clone()
    };
    for units in [1, MAX_SURFACE_RULE_BIND_WORK] {
        assert_eq!(
            refuse(Vanilla, Disabled, units),
            SurfaceRuleErrorKind::UnsupportedNumericProfile
        );
        assert_eq!(
            refuse(Vanilla, Enabled, units),
            SurfaceRuleErrorKind::UnsupportedNumericProfile
        );
        assert_eq!(
            refuse(PaperBuild60, Enabled, units),
            SurfaceRuleErrorKind::UnsupportedFlatBedrock
        );
    }
    assert_eq!(
        refuse(PaperBuild60, Disabled, 1),
        SurfaceRuleErrorKind::WorkBudgetExceeded
    );
    for kind in [
        SurfaceRuleErrorKind::UnsupportedNumericProfile,
        SurfaceRuleErrorKind::UnsupportedFlatBedrock,
    ] {
        assert_eq!(kind.category(), SurfaceRuleRefusalCategory::Policy);
    }
}

/// P3: only `overworld`, `large_biomes` and `amplified` compile; the census
/// allowance is checked before the settings admission.
#[test]
fn only_the_three_overworld_settings_are_admitted() {
    for name in [
        "minecraft:caves",
        "minecraft:floating_islands",
        "minecraft:nether",
        "minecraft:end",
    ] {
        let error = SurfaceRuleTemplate::compile_builtin(
            settings(name),
            build_budget(MAX_SURFACE_RULE_BUILD_WORK),
        )
        .unwrap_err();
        assert_eq!(
            *error.kind(),
            SurfaceRuleErrorKind::UnsupportedSettings,
            "{name}"
        );
        let error =
            SurfaceRuleTemplate::compile_builtin(settings(name), build_budget(1)).unwrap_err();
        assert_eq!(
            *error.kind(),
            SurfaceRuleErrorKind::WorkBudgetExceeded,
            "{name}"
        );
    }
    for name in [
        "minecraft:overworld",
        "minecraft:large_biomes",
        "minecraft:amplified",
    ] {
        assert_eq!(template(settings(name)).resource(), settings(name));
    }
}

/// P6 and consistency (A8, R2-H5, R3-H3): one mismatching field at a time in
/// the order Seed, Settings, Source, Geometry, `ZoomSemantics`. The level height
/// assumption `(-64, 384)` of the admitted settings is what `Geometry` checks.
#[test]
fn resolver_mismatches_are_refused_in_identity_order() {
    use ResolverMismatch::{Geometry, Seed, Settings, Source, ZoomSemantics};
    let (group, generated) = origin();
    let refs = generated.refs();
    let paper = resolver(generated, &refs);
    let vanilla = SurfaceBiomeResolver::new(
        generated.identity,
        group.center,
        &refs,
        BiomeZoomSemantics::Vanilla1_21_8,
        stored_budget(),
    )
    .unwrap();
    let rules = binding(group);
    let other_seed = bind(overworld(), 1);
    let other_settings = bind(template(settings("minecraft:large_biomes")), 0);
    let mismatch = |kind: SurfaceRuleErrorKind| match kind {
        SurfaceRuleErrorKind::InconsistentResolver(m) => m,
        other => panic!("{other:?}"),
    };
    assert_eq!(
        geometry_of(&paper),
        (SURFACE_RULE_LEVEL_MIN_Y, SURFACE_RULE_LEVEL_HEIGHT)
    );
    assert!(
        SurfaceRuleSession::new(rules, &paper, temperatures(), group.corners, work_budget(1))
            .is_ok()
    );
    assert_eq!(mismatch(session_error(&other_seed, &paper)), Seed);
    assert_eq!(mismatch(session_error(&other_settings, &paper)), Settings);
    assert_eq!(mismatch(session_error(rules, &vanilla)), ZoomSemantics);
    // Earlier fields win over the zoom semantics and over later fields.
    assert_eq!(mismatch(session_error(&other_seed, &vanilla)), Seed);
    assert_eq!(mismatch(session_error(&other_settings, &vanilla)), Settings);
    let nether = BiomeSourceIdentity::MultiNoise(
        BiomeParametersResourceId::from_name("minecraft:nether").unwrap(),
    );
    let plains =
        BiomeSourceIdentity::Fixed(BiomeResourceId::from_name("minecraft:plains").unwrap());
    let shifted = BiomeQuartGeometry::new(-80, 416).unwrap();
    let taller = BiomeQuartGeometry::new(-64, 400).unwrap();
    let cases = [
        (plains, level_geometry(), Source),
        (BiomeSourceIdentity::End, level_geometry(), Source),
        (nether, level_geometry(), Source),
        (overworld_source(), shifted, Geometry),
        (overworld_source(), taller, Geometry),
        (BiomeSourceIdentity::End, shifted, Source),
    ];
    for (source, geometry, expected) in cases {
        let arrays = arrays(source, geometry);
        let refs: Vec<_> = arrays.iter().collect();
        for semantics in [
            BiomeZoomSemantics::Paper1_21_8Build60,
            BiomeZoomSemantics::Vanilla1_21_8,
        ] {
            let resolver = SurfaceBiomeResolver::new(
                identity(source, geometry),
                group.center,
                &refs,
                semantics,
                stored_budget(),
            )
            .unwrap();
            assert_eq!(
                mismatch(session_error(rules, &resolver)),
                expected,
                "{source:?} {geometry:?}"
            );
            assert_eq!(
                mismatch(session_error(&other_settings, &resolver)),
                Settings
            );
            assert_eq!(mismatch(session_error(&other_seed, &resolver)), Seed);
        }
    }
    for m in [Seed, Settings, Geometry] {
        assert_eq!(
            SurfaceRuleErrorKind::InconsistentResolver(m).category(),
            SurfaceRuleRefusalCategory::Input
        );
    }
    for m in [Source, ZoomSemantics] {
        assert_eq!(
            SurfaceRuleErrorKind::InconsistentResolver(m).category(),
            SurfaceRuleRefusalCategory::Policy
        );
    }
}

fn geometry_of(resolver: &SurfaceBiomeResolver<'_>) -> (i32, u32) {
    let geometry = resolver.identity().geometry();
    (geometry.min_y(), geometry.height())
}

type DomainCase = (i32, i32, i32, Option<i32>, Option<SurfaceRuleErrorKind>);

/// P4: the `y`, stone-depth and water domains in the `evaluate` order, all
/// refused before any mutation; the session stays usable.
#[test]
fn block_domains_are_refused_in_order_without_effect() {
    use SurfaceRuleErrorKind::{
        BlockOutsideGenerationHeight as Height, InvalidStoneDepth as Stone,
        InvalidWaterHeight as Water,
    };
    let (group, generated) = origin();
    let refs = generated.refs();
    let resolver = resolver(generated, &refs);
    let rules = binding(group);
    let mut session = SurfaceRuleSession::new(
        rules,
        &resolver,
        temperatures(),
        group.corners,
        work_budget(1_000_000),
    )
    .unwrap();
    session
        .begin_column(SurfaceColumnPos::new(group.center, 7, 9).unwrap())
        .unwrap();
    let state = session.projection();
    let cases: [DomainCase; 16] = [
        (-65, 1, 1, None, Some(Height)),
        (-64, 1, 1, None, None),
        (319, 1, 1, None, None),
        (320, 1, 1, None, Some(Height)),
        (i32::MIN, 0, 0, Some(i32::MIN), Some(Height)),
        (i32::MAX, 1, 1, None, Some(Height)),
        (64, 0, 1, None, Some(Stone)),
        (64, 4097, 1, None, Some(Stone)),
        (64, 1, 0, None, Some(Stone)),
        (64, 1, 4097, None, Some(Stone)),
        (64, 4096, 4096, None, None),
        (64, 0, 1, Some(i32::MIN), Some(Stone)),
        (64, 1, 1, Some(i32::MIN), Some(Water)),
        (64, 1, 1, Some(i32::MIN + 1), None),
        (64, 1, 1, Some(i32::MAX), None),
        (64, 1, 1, None, None),
    ];
    for (y, above, below, water, expected) in cases {
        let input = SurfaceBlockInput::new(y, above, below, water);
        let before = (session.projection(), session.block);
        match (session.evaluate(input, &group.heights), expected) {
            (Ok(_), None) => {}
            (Err(error), Some(kind)) => {
                assert_eq!(*error.kind(), kind, "{input:?}");
                assert_eq!(error.block(), Some(BlockPos::new(7, y, 9)));
                // No mutation at all, including the per-block state of the previous call.
                assert_eq!((session.projection(), session.block), before, "{input:?}");
                assert_eq!(kind.category(), SurfaceRuleRefusalCategory::Policy);
            }
            (other, expected) => panic!("{input:?}: {other:?} vs {expected:?}"),
        }
    }
    assert_ne!(session.projection(), state);
}

/// Column domain and active-column checks are input refusals without effect.
#[test]
fn column_outside_centre_and_no_active_column() {
    let (group, generated) = origin();
    let refs = generated.refs();
    let resolver = resolver(generated, &refs);
    let mut session = SurfaceRuleSession::new(
        binding(group),
        &resolver,
        temperatures(),
        group.corners,
        work_budget(10_000),
    )
    .unwrap();
    let input = SurfaceBlockInput::new(64, 1, 1, None);
    let none = SurfaceRuleErrorKind::NoActiveColumn;
    assert_eq!(*session.surface_depth().unwrap_err().kind(), none);
    assert_eq!(*session.min_surface_level().unwrap_err().kind(), none);
    assert_eq!(
        *session.evaluate(input, &group.heights).unwrap_err().kind(),
        none
    );
    for chunk in [
        ChunkPos::new(1, 0),
        ChunkPos::new(0, -1),
        ChunkPos::new(-1, -1),
        ChunkPos::new(i32::MAX, 0),
    ] {
        let error = session
            .begin_column(SurfaceColumnPos::new(chunk, 0, 0).unwrap())
            .unwrap_err();
        assert_eq!(*error.kind(), SurfaceRuleErrorKind::ColumnOutsideCentre);
    }
    assert_eq!(session.work_used(), 0);
    assert_eq!(session.projection(), (None, 0, false));
    assert!(SurfaceColumnPos::new(group.center, 16, 0).is_none());
    assert!(SurfaceColumnPos::new(group.center, 0, 16).is_none());
    let from = SurfaceColumnPos::from_block(BlockPos::new(-1, 70, -17));
    assert_eq!(
        (from.chunk(), from.local_x(), from.local_z()),
        (ChunkPos::new(-1, -2), 15, 15)
    );
    assert_eq!((from.block_x(), from.block_z()), (-1, -17));
    for kind in [SurfaceRuleErrorKind::ColumnOutsideCentre, none] {
        assert_eq!(kind.category(), SurfaceRuleRefusalCategory::Input);
    }
}

/// Every kind belongs to exactly the documented group.
#[test]
fn refusal_categories_group_policy_limit_input_and_defensive() {
    use SurfaceRuleErrorKind as K;
    use SurfaceRuleRefusalCategory::{Defensive, Input, Limit, Policy};
    let cases = [
        (K::InvalidBudget, Limit),
        (K::NativeLimit, Limit),
        (K::UnsupportedSettings, Policy),
        (K::UnsupportedNumericProfile, Policy),
        (K::UnsupportedFlatBedrock, Policy),
        (K::BlockOutsideGenerationHeight, Policy),
        (K::InvalidStoneDepth, Policy),
        (K::InvalidWaterHeight, Policy),
        (K::WorkBudgetExceeded, Policy),
        (K::Poisoned, Policy),
        (K::MalformedDefinition, Input),
        (K::UnknownCodecType, Input),
        (K::InvalidBlockState, Input),
        (K::UnknownBiome, Input),
        (K::UnknownNoise, Input),
        (K::InvalidAnchor, Input),
        (K::ColumnOutsideCentre, Input),
        (K::NoActiveColumn, Input),
        (K::Noise, Defensive),
        (
            K::Biome(crate::stored_biome::StoredBiomeErrorKind::MissingChunk),
            Defensive,
        ),
        (
            K::Temperature(crate::biome_temperature::BiomeTemperatureErrorKind::InvalidResource),
            Defensive,
        ),
    ];
    for (kind, category) in cases {
        assert_eq!(kind.category(), category, "{kind:?}");
    }
}
