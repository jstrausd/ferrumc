//! Shared fixture parsing, genuine producers and the mechanical trace projection.
//!
//! Original expectations come only from the compact mechanical projections in
//! `tests/fixtures/surface-rules-original-1.21.8` and
//! `tests/fixtures/surface-rules-clay-1.21.8`. Paper cells are expectations for
//! `PaperFlatBedrock::Disabled`; Vanilla cells are discriminating evidence only.
//! Policy and malformed-input tests live in separate modules and never read
//! original rows as expectations.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::{Arc, OnceLock};

use ferrumc_math::{BlockPos, ChunkPos, Vec3};
use ferrumc_registry::biome::{BiomeParametersResourceId, BiomeResourceId};
use ferrumc_registry::worldgen::NoiseSettingsResourceId;
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::compile::NodeKind;
use super::session::Column;
use super::*;
use crate::biome_selection::{
    BiomeBuildBudget, BiomeQuartGeometry, BiomeSelectionSession, BiomeSourceIdentity,
    BiomeSourceSession, BiomeTree, BiomeWorkBudget, ChunkBiomeSamples, MAX_BIOME_BUILD_WORK,
    MAX_BIOME_SELECTION_WORK,
};
use crate::biome_temperature::{BiomeTemperatureBuildBudget, BuiltinBiomeTemperatures};
use crate::biome_zoom::BiomeZoomSemantics;
use crate::density::{NoiseChunkWorkBudget, RouterTemplate, MAX_NOISE_CHUNK_WORK};
use crate::noise::NormalNoise;
use crate::random::{RandomAlgorithm, WorldgenRandomSource};
use crate::stored_biome::{
    StoredBiomeBuildBudget, StoredBiomeIdentity, SurfaceBiomeResolver, MAX_STORED_BIOME_BUILD_WORK,
};
use crate::WorldSeed;

mod allocation;
mod basis;
mod limits;
mod malformed;
mod oracle;
mod policy;
mod steep;
mod tree;
mod work;

const MANIFEST: &str =
    include_str!("../../tests/fixtures/surface-rules-original-1.21.8/manifest.json");
const ROWS: &str = include_str!("../../tests/fixtures/surface-rules-original-1.21.8/rows.tsv");
const REPLAY: &str = include_str!("../../tests/fixtures/surface-rules-original-1.21.8/replay.tsv");
const GROUPS: &str = include_str!("../../tests/fixtures/surface-rules-original-1.21.8/groups.tsv");
const ARRAYS: &str = include_str!("../../tests/fixtures/surface-rules-original-1.21.8/arrays.tsv");
const COLUMNS: &str =
    include_str!("../../tests/fixtures/surface-rules-original-1.21.8/columns.tsv");
const PATTERNS: &str =
    include_str!("../../tests/fixtures/surface-rules-original-1.21.8/patterns.tsv");
const STATES: &str = include_str!("../../tests/fixtures/surface-rules-original-1.21.8/states.tsv");
const BIOMES: &str = include_str!("../../tests/fixtures/surface-rules-original-1.21.8/biomes.tsv");
const BANDS: &str = include_str!("../../tests/fixtures/surface-rules-original-1.21.8/bands.tsv");
const STEEP: &str =
    include_str!("../../tests/fixtures/surface-rules-original-1.21.8/steep-heights.tsv");
pub(super) const TREE: &str =
    include_str!("../../tests/fixtures/surface-rules-original-1.21.8/tree.tsv");
pub(super) const TREE_PAPER: &str =
    include_str!("../../tests/fixtures/surface-rules-original-1.21.8/tree-paper.tsv");
pub(super) const SUBSTITUTION: &str =
    include_str!("../../tests/fixtures/surface-rules-original-1.21.8/paper-tree-substitution.json");
pub(super) const CLAY: &str =
    include_str!("../../tests/fixtures/surface-rules-clay-1.21.8/clay.tsv");
pub(super) const CLAY_MANIFEST: &str =
    include_str!("../../tests/fixtures/surface-rules-clay-1.21.8/manifest.json");

/// Fixture file names of the frozen 14-file projection (manifest `files` plus itself).
pub(super) const FIXTURE_FILES: [&str; 13] = [
    "arrays.tsv",
    "bands.tsv",
    "biomes.tsv",
    "columns.tsv",
    "groups.tsv",
    "paper-tree-substitution.json",
    "patterns.tsv",
    "replay.tsv",
    "rows.tsv",
    "states.tsv",
    "steep-heights.tsv",
    "tree-paper.tsv",
    "tree.tsv",
];

pub(super) struct Group {
    pub gid: usize,
    pub id: String,
    pub settings: NoiseSettingsResourceId,
    pub seed: i64,
    pub center: ChunkPos,
    pub corners: PreliminarySurfaceCorners,
    pub bands: Vec<String>,
    pub anchors: String,
    pub heights: WorldSurfaceHeights,
    pub min_gen_y: i32,
    pub gen_depth: i32,
    pub sea_level: i32,
}

pub(super) struct Row {
    pub ordinal: usize,
    pub gid: usize,
    pub tier: u8,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub surface_depth: i32,
    pub stone_above: i32,
    pub stone_below: i32,
    pub water: Option<i32>,
    pub result: usize,
    pub biome: usize,
    pub pattern: usize,
    pub steep_heights: String,
    pub min_surface: String,
    pub secondary: String,
    pub column: Option<usize>,
    pub noise_values: String,
    pub gradient_draws: String,
}

impl Row {
    pub const fn input(&self) -> SurfaceBlockInput {
        SurfaceBlockInput::new(self.y, self.stone_above, self.stone_below, self.water)
    }
    pub const fn column_pos(&self) -> SurfaceColumnPos {
        SurfaceColumnPos::from_block(BlockPos::new(self.x, self.y, self.z))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Class {
    Full,
    Partial,
    Refused,
}

pub(super) struct Replay {
    pub ordinal: usize,
    pub run_start: usize,
    pub class: Class,
    pub steep_probe: bool,
    pub refusal: Option<SurfaceRuleErrorKind>,
}

pub(super) struct Fixture {
    pub manifest: Value,
    pub groups: Vec<Group>,
    pub rows: Vec<Row>,
    pub by_ordinal: HashMap<usize, usize>,
    pub replay: Vec<Replay>,
    pub states: Vec<String>,
    pub biomes: Vec<BiomeResourceId>,
    pub patterns: Vec<String>,
    pub columns: Vec<String>,
    pub steep: HashMap<usize, WorldSurfaceHeights>,
    pub arrays: Vec<(usize, ChunkPos, String)>,
    pub diagnostics: usize,
}

fn table(text: &str, columns: usize) -> impl Iterator<Item = Vec<&str>> {
    let mut lines = text.lines();
    assert_eq!(lines.next().unwrap().split('\t').count(), columns);
    lines.map(move |line| {
        let row: Vec<_> = line.split('\t').collect();
        assert_eq!(row.len(), columns, "{line}");
        row
    })
}

fn heights(values: &str) -> WorldSurfaceHeights {
    let flat: Vec<i32> = values.split(',').map(|v| v.parse().unwrap()).collect();
    assert_eq!(flat.len(), 256);
    WorldSurfaceHeights::new(std::array::from_fn(|x| {
        std::array::from_fn(|z| flat[x * 16 + z])
    }))
}

fn settings_of(preset: &str) -> NoiseSettingsResourceId {
    NoiseSettingsResourceId::from_name(match preset {
        "normal" => "minecraft:overworld",
        "large_biomes" => "minecraft:large_biomes",
        "amplified" => "minecraft:amplified",
        other => panic!("unsupported preset {other}"),
    })
    .unwrap()
}

fn parse_groups() -> Vec<Group> {
    let bands: Vec<String> = table(BANDS, 2).map(|r| r[1].to_owned()).collect();
    table(GROUPS, 16)
        .map(|r| {
            let corners: Vec<i32> = r[8].split(',').map(|v| v.parse().unwrap()).collect();
            Group {
                gid: r[0].parse().unwrap(),
                id: r[1].to_owned(),
                settings: settings_of(r[2]),
                seed: r[3].parse().unwrap(),
                center: ChunkPos::new(r[4].parse().unwrap(), r[5].parse().unwrap()),
                corners: PreliminarySurfaceCorners::new(
                    corners[0], corners[1], corners[2], corners[3],
                ),
                bands: r[12]
                    .split(',')
                    .map(|i| bands[i.parse::<usize>().unwrap()].clone())
                    .collect(),
                anchors: r[13].to_owned(),
                heights: heights(r[14]),
                sea_level: r[9].parse().unwrap(),
                min_gen_y: r[10].parse().unwrap(),
                gen_depth: r[11].parse().unwrap(),
            }
        })
        .collect()
}

fn parse_row(r: &[&str]) -> Row {
    let optional = |cell: &str| (cell != "-").then(|| cell.parse::<usize>().unwrap());
    Row {
        ordinal: r[0].parse().unwrap(),
        gid: r[1].parse().unwrap(),
        tier: r[2].as_bytes()[0],
        x: r[3].parse().unwrap(),
        y: r[4].parse().unwrap(),
        z: r[5].parse().unwrap(),
        surface_depth: r[6].parse().unwrap(),
        stone_above: r[7].parse().unwrap(),
        stone_below: r[8].parse().unwrap(),
        water: (r[9] != "-").then(|| r[9].parse().unwrap()),
        result: r[10].parse().unwrap(),
        biome: r[11].parse().unwrap(),
        pattern: r[12].parse().unwrap(),
        steep_heights: r[13].to_owned(),
        min_surface: r[14].to_owned(),
        secondary: r[15].to_owned(),
        column: optional(r[16]),
        noise_values: r[17].to_owned(),
        gradient_draws: r[18].to_owned(),
    }
}

fn parse_replay() -> Vec<Replay> {
    table(REPLAY, 5)
        .map(|r| Replay {
            ordinal: r[0].parse().unwrap(),
            run_start: r[1].parse().unwrap(),
            class: match r[2] {
                "full" => Class::Full,
                "partial" => Class::Partial,
                "refused" => Class::Refused,
                other => panic!("unknown replay class {other}"),
            },
            steep_probe: match r[3] {
                "steep-probe" => true,
                "final" => false,
                other => panic!("unknown heights view {other}"),
            },
            refusal: match r[4] {
                "-" => None,
                "BlockOutsideGenerationHeight" => {
                    Some(SurfaceRuleErrorKind::BlockOutsideGenerationHeight)
                }
                "InvalidStoneDepth" => Some(SurfaceRuleErrorKind::InvalidStoneDepth),
                "InvalidWaterHeight" => Some(SurfaceRuleErrorKind::InvalidWaterHeight),
                other => panic!("unknown refusal {other}"),
            },
        })
        .collect()
}

fn parse() -> Fixture {
    let manifest: Value = serde_json::from_str(MANIFEST).unwrap();
    let mut rows = Vec::new();
    let mut diagnostics = 0;
    for r in table(ROWS, 20) {
        if r[2] == "D" {
            diagnostics += 1;
        } else {
            rows.push(parse_row(&r));
        }
    }
    let by_ordinal = rows
        .iter()
        .enumerate()
        .map(|(i, r)| (r.ordinal, i))
        .collect();
    let steep = table(STEEP, 2)
        .map(|r| (r[0].parse().unwrap(), heights(r[1])))
        .collect();
    let arrays = table(ARRAYS, 5)
        .map(|r| {
            (
                r[0].parse().unwrap(),
                ChunkPos::new(r[1].parse().unwrap(), r[2].parse().unwrap()),
                r[3].to_owned(),
            )
        })
        .collect();
    Fixture {
        groups: parse_groups(),
        by_ordinal,
        replay: parse_replay(),
        states: table(STATES, 2).map(|r| r[1].to_owned()).collect(),
        biomes: table(BIOMES, 2)
            .map(|r| BiomeResourceId::from_name(r[1]).unwrap())
            .collect(),
        patterns: table(PATTERNS, 2).map(|r| r[1].to_owned()).collect(),
        columns: table(COLUMNS, 5).map(|r| r[4].to_owned()).collect(),
        steep,
        arrays,
        diagnostics,
        rows,
        manifest,
    }
}

pub(super) fn fixture() -> &'static Fixture {
    static FIXTURE: OnceLock<Fixture> = OnceLock::new();
    FIXTURE.get_or_init(parse)
}

pub(super) fn settings(name: &str) -> NoiseSettingsResourceId {
    NoiseSettingsResourceId::from_name(name).unwrap()
}

pub(super) fn build_budget(units: u32) -> SurfaceRuleBuildBudget {
    SurfaceRuleBuildBudget::new(units).unwrap()
}

pub(super) fn bind_budget(units: u32) -> SurfaceRuleBindBudget {
    SurfaceRuleBindBudget::new(units).unwrap()
}

pub(super) fn work_budget(units: u32) -> SurfaceRuleWorkBudget {
    SurfaceRuleWorkBudget::new(units).unwrap()
}

/// One compiled template per admitted settings resource.
pub(super) fn template(resource: NoiseSettingsResourceId) -> Arc<SurfaceRuleTemplate> {
    static TEMPLATES: OnceLock<Vec<(NoiseSettingsResourceId, Arc<SurfaceRuleTemplate>)>> =
        OnceLock::new();
    let all = TEMPLATES.get_or_init(|| {
        [
            "minecraft:overworld",
            "minecraft:large_biomes",
            "minecraft:amplified",
        ]
        .into_iter()
        .map(|name| {
            let resource = settings(name);
            let template = SurfaceRuleTemplate::compile_builtin(
                resource,
                build_budget(MAX_SURFACE_RULE_BUILD_WORK),
            )
            .unwrap();
            (resource, Arc::new(template))
        })
        .collect()
    });
    Arc::clone(&all.iter().find(|(r, _)| *r == resource).unwrap().1)
}

pub(super) fn overworld() -> Arc<SurfaceRuleTemplate> {
    template(settings("minecraft:overworld"))
}

pub(super) fn bind(template: Arc<SurfaceRuleTemplate>, seed: i64) -> SeededSurfaceRules {
    SeededSurfaceRules::bind(
        template,
        WorldSeed::new(seed),
        SurfaceNumericProfile::PaperBuild60,
        PaperFlatBedrock::Disabled,
        bind_budget(MAX_SURFACE_RULE_BIND_WORK),
    )
    .unwrap()
}

pub(super) fn temperatures() -> &'static BuiltinBiomeTemperatures {
    static TABLE: OnceLock<BuiltinBiomeTemperatures> = OnceLock::new();
    TABLE.get_or_init(|| {
        BuiltinBiomeTemperatures::compile(BiomeTemperatureBuildBudget::new(1_048_576).unwrap())
            .unwrap()
    })
}

pub(super) fn overworld_source() -> BiomeSourceIdentity {
    BiomeSourceIdentity::MultiNoise(
        BiomeParametersResourceId::from_name("minecraft:overworld").unwrap(),
    )
}

pub(super) fn level_geometry() -> BiomeQuartGeometry {
    BiomeQuartGeometry::new(SURFACE_RULE_LEVEL_MIN_Y, SURFACE_RULE_LEVEL_HEIGHT).unwrap()
}

pub(super) fn stored_budget() -> StoredBiomeBuildBudget {
    StoredBiomeBuildBudget::new(MAX_STORED_BIOME_BUILD_WORK).unwrap()
}

/// Genuine GEN-05.5 arrays of one group: cold history, nine chunks row-major (x outer).
pub(super) struct Generated {
    pub gid: usize,
    pub identity: StoredBiomeIdentity,
    pub arrays: Vec<ChunkBiomeSamples>,
}

impl Generated {
    pub fn refs(&self) -> Vec<&ChunkBiomeSamples> {
        self.arrays.iter().collect()
    }
}

fn produce() -> Vec<Generated> {
    let fixture = fixture();
    let tree = Arc::new(
        BiomeTree::from_builtin(
            BiomeParametersResourceId::from_name("minecraft:overworld").unwrap(),
            BiomeBuildBudget::new(MAX_BIOME_BUILD_WORK).unwrap(),
        )
        .unwrap(),
    );
    let mut routers = HashMap::new();
    let mut generated = Vec::new();
    for group in &fixture.groups {
        let identity = StoredBiomeIdentity::new(
            WorldSeed::new(group.seed),
            group.settings,
            overworld_source(),
            level_geometry(),
        );
        let router = routers
            .entry((group.settings, group.seed))
            .or_insert_with(|| {
                RouterTemplate::compile_builtin(group.settings)
                    .unwrap()
                    .bind_seed(WorldSeed::new(group.seed))
                    .unwrap()
            });
        // One cold source session per group; its nine chunks share the history.
        let mut session = BiomeSourceSession::from_multinoise(BiomeSelectionSession::new(
            Arc::clone(&tree),
            BiomeWorkBudget::new(MAX_BIOME_SELECTION_WORK).unwrap(),
        ));
        let mut arrays = Vec::with_capacity(9);
        for dx in -1..=1 {
            for dz in -1..=1 {
                let chunk = ChunkPos::new(group.center.x() + dx, group.center.z() + dz);
                let mut noise = router
                    .fresh_paper_noise_chunk(
                        chunk,
                        NoiseChunkWorkBudget::new(MAX_NOISE_CHUNK_WORK).unwrap(),
                    )
                    .unwrap();
                arrays.push(
                    session
                        .sample_chunk_biomes(&mut noise, level_geometry())
                        .unwrap(),
                );
            }
        }
        generated.push(Generated {
            gid: group.gid,
            identity,
            arrays,
        });
    }
    generated
}

pub(super) fn generated() -> &'static [Generated] {
    static GENERATED: OnceLock<Vec<Generated>> = OnceLock::new();
    GENERATED.get_or_init(produce)
}

pub(super) fn resolver<'a>(
    generated: &Generated,
    refs: &[&'a ChunkBiomeSamples],
) -> SurfaceBiomeResolver<'a> {
    let center = fixture().groups[generated.gid].center;
    SurfaceBiomeResolver::new(
        generated.identity,
        center,
        refs,
        BiomeZoomSemantics::Paper1_21_8Build60,
        stored_budget(),
    )
    .unwrap()
}

/// Bindings per (settings, seed), built once.
pub(super) fn binding(group: &Group) -> &'static SeededSurfaceRules {
    static BINDINGS: OnceLock<Vec<((NoiseSettingsResourceId, i64), SeededSurfaceRules)>> =
        OnceLock::new();
    let all = BINDINGS.get_or_init(|| {
        let mut out: Vec<((NoiseSettingsResourceId, i64), SeededSurfaceRules)> = Vec::new();
        for g in &fixture().groups {
            if !out.iter().any(|(k, _)| *k == (g.settings, g.seed)) {
                out.push(((g.settings, g.seed), bind(template(g.settings), g.seed)));
            }
        }
        out
    });
    &all.iter()
        .find(|(k, _)| *k == (group.settings, group.seed))
        .unwrap()
        .1
}

/// The Vanilla-basis sampler of a binding noise slot (discriminating control only).
pub(super) fn vanilla_noise(seed: i64, name: &str) -> NormalNoise {
    let id = ferrumc_registry::worldgen::NoiseResourceId::from_name(name).unwrap();
    let root =
        WorldgenRandomSource::new(RandomAlgorithm::Xoroshiro128PlusPlus, WorldSeed::new(seed))
            .fork_positional();
    NormalNoise::new(
        &mut root.from_hash_of(id.resource_location()),
        id.parameters().first_octave(),
        id.parameters().amplitudes(),
    )
    .unwrap()
}

pub(super) fn sample_bits(noise: &NormalNoise, x: i32, z: i32) -> u64 {
    noise
        .sample(Vec3::new(f64::from(x), 0.0, f64::from(z)))
        .unwrap()
        .to_bits()
}

/// Java 19+ `Double.toString`: shortest digits, plain in `[1e-3, 1e7)`, else `d.dddE±n`.
pub(super) fn java_double(value: f64) -> String {
    if value == 0.0 {
        return if value.is_sign_negative() {
            "-0.0"
        } else {
            "0.0"
        }
        .to_owned();
    }
    let sign = if value < 0.0 { "-" } else { "" };
    let text = format!("{:e}", value.abs());
    let (mantissa, exponent) = text.split_once('e').unwrap();
    let exponent: i32 = exponent.parse().unwrap();
    let mut digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    // Java picks the even last digit when the exact value lies exactly halfway
    // between two shortest candidates; the Rust shortest formatter does not.
    let exact = format!("{:.80e}", value.abs());
    let exact: String = exact
        .split_once('e')
        .unwrap()
        .0
        .chars()
        .filter(char::is_ascii_digit)
        .collect();
    let exact = exact.trim_end_matches('0');
    if exact.len() == digits.len() + 1
        && exact.ends_with('5')
        && exact.starts_with(&digits[..digits.len() - 1])
    {
        let lower = &exact[..digits.len()];
        let last = lower.as_bytes()[lower.len() - 1];
        let upper_last = last + 1;
        let even = if (last - b'0').is_multiple_of(2) {
            last
        } else {
            upper_last
        };
        if upper_last <= b'9' {
            let candidate = format!("{}{}", &lower[..lower.len() - 1], char::from(even));
            let check: f64 = format!("{}.{}e{exponent}", &candidate[..1], &candidate[1..])
                .parse()
                .unwrap_or(f64::NAN);
            if check.to_bits() == value.abs().to_bits() {
                digits = candidate;
            }
        }
    }
    let magnitude = value.abs();
    if (1e-3..1e7).contains(&magnitude) {
        let point = exponent + 1;
        let body = if point <= 0 {
            format!("0.{}{digits}", "0".repeat((-point) as usize))
        } else if point as usize >= digits.len() {
            format!("{digits}{}.0", "0".repeat(point as usize - digits.len()))
        } else {
            format!(
                "{}.{}",
                &digits[..point as usize],
                &digits[point as usize..]
            )
        };
        format!("{sign}{body}")
    } else {
        let fraction = if digits.len() > 1 { &digits[1..] } else { "0" };
        format!("{sign}{}.{fraction}E{exponent}", &digits[..1])
    }
}

/// Original `doubleText`: `Double.toString(v) + "/" + Long.toHexString(bits)`.
pub(super) fn double_text(value: f64) -> String {
    format!("{}/{:x}", java_double(value), value.to_bits())
}

/// Original `ChunkPos.asLong`.
pub(super) fn chunk_long(chunk: ChunkPos) -> i64 {
    (i64::from(chunk.x()) & 0xffff_ffff) | ((i64::from(chunk.z()) & 0xffff_ffff) << 32)
}

/// Per-block node kinds whose original laziness is per `updateY`.
pub(super) fn per_block(kind: NodeKind) -> bool {
    matches!(
        kind,
        NodeKind::Biome { .. }
            | NodeKind::VerticalGradient(_)
            | NodeKind::PaperFlatBedrock(_)
            | NodeKind::YAbove(_)
            | NodeKind::Water(_)
            | NodeKind::StoneDepth(_)
    )
}

/// Runtime projection of one `evaluate` call in the oracle's row tokens.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Trace {
    pub computed: String,
    pub singles: String,
    pub noise_values: String,
    pub gradient_draws: String,
    pub steep_heights: String,
    pub min_surface: String,
    pub secondary: String,
}

/// Projects one completed call from the session state before and after it.
#[allow(clippy::many_single_char_names)] // original block coordinates x/y/z
pub(super) fn trace(
    rules: &SeededSurfaceRules,
    before: Option<Column>,
    session: &SurfaceRuleSession<'_>,
    corners: PreliminarySurfaceCorners,
    y: i32,
) -> Trace {
    let template = rules.template();
    let after = session.column.unwrap();
    let block = session.block;
    let (x, z) = (after.pos.block_x(), after.pos.block_z());
    let mut computed = Vec::new();
    let (mut noise_values, mut gradient_draws) = (Vec::new(), Vec::new());
    for (id, node) in template.nodes.iter().enumerate() {
        let (done, value) = if per_block(node.kind) {
            (block.done.get(id), block.value.get(id))
        } else if matches!(node.kind, NodeKind::NoiseThreshold(_)) {
            let fresh = after.noise_done.get(id) && !before.is_some_and(|b| b.noise_done.get(id));
            (fresh, after.noise_value.get(id))
        } else {
            (false, false)
        };
        if !done {
            continue;
        }
        computed.push(format!("{id}:{}", u8::from(value)));
        match node.kind {
            NodeKind::NoiseThreshold(t) => {
                let slot = usize::from(template.thresholds[usize::from(t)].noise);
                noise_values.push(format!("{id}={:x}", sample_bits(&rules.noises[slot], x, z)));
            }
            NodeKind::VerticalGradient(g) | NodeKind::PaperFlatBedrock(g) => {
                let random = usize::from(template.gradients[usize::from(g)].random);
                let draw = rules.factories[random]
                    .at(BlockPos::new(x, y, z))
                    .next_float();
                gradient_draws.push(format!("{id}={:x}", draw.to_bits()));
            }
            _ => {}
        }
    }
    let mut singles = Vec::new();
    let mut steep_heights = "-".to_owned();
    if let Some((steep, read)) = after
        .steep
        .filter(|_| before.is_none_or(|b| b.steep.is_none()))
    {
        singles.push(format!("steep:{}", u8::from(steep)));
        steep_heights = read.map(|h| h.to_string()).join(",");
    }
    if let Some(hole) = after
        .hole
        .filter(|_| before.is_none_or(|b| b.hole.is_none()))
    {
        singles.push(format!("hole:{}", u8::from(hole)));
    }
    if let Some(cold) = block.temperature {
        singles.push(format!("temperature:{}", u8::from(cold)));
    }
    if let Some(min) = after.min_surface {
        singles.push(format!("above:{}", u8::from(y >= min)));
    }
    let min_surface = match after.min_surface {
        Some(min) if before.is_none_or(|b| b.min_surface.is_none()) => {
            let [a, b, c, d] = corners.values();
            format!("{min};{a},{b},{c},{d};{}", chunk_long(after.pos.chunk()))
        }
        _ => "-".to_owned(),
    };
    let secondary = match after.secondary.get() {
        Some(value) if before.is_none_or(|b| b.secondary.is_none()) => double_text(value),
        _ => "-".to_owned(),
    };
    let or_dash = |list: Vec<String>| {
        if list.is_empty() {
            "-".to_owned()
        } else {
            list.join(",")
        }
    };
    Trace {
        computed: computed.join(","),
        singles: or_dash(singles),
        noise_values: or_dash(noise_values),
        gradient_draws: or_dash(gradient_draws),
        steep_heights,
        min_surface,
        secondary,
    }
}

/// Original result token of an outcome (`states.tsv` form) — compares the state id only.
pub(super) fn outcome_matches(outcome: SurfaceRuleOutcome, expected: &str) -> bool {
    match outcome {
        SurfaceRuleOutcome::NoRule => expected == "none",
        SurfaceRuleOutcome::State(state) => {
            expected.split('|').nth(1) == Some(state.as_u32().to_string().as_str())
        }
    }
}

/// Array digest exactly as the generator: SHA-256 of comma-joined roster indices.
pub(super) fn array_digest(values: &[BiomeResourceId]) -> String {
    let mut roster: Vec<&str> = ferrumc_registry::biome::biome_resources()
        .map(BiomeResourceId::resource_location)
        .collect();
    roster.sort_unstable();
    let mut text = String::new();
    for (i, value) in values.iter().enumerate() {
        if i > 0 {
            text.push(',');
        }
        let index = roster.binary_search(&value.resource_location()).unwrap();
        write!(text, "{index}").unwrap();
    }
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

/// The fixture is the frozen round-3 projection, byte for byte (manifest digests).
#[test]
fn fixture_files_match_their_manifest_and_counts() {
    let fixture = fixture();
    let files = fixture.manifest["files"].as_object().unwrap();
    assert_eq!(files.len(), FIXTURE_FILES.len());
    for name in FIXTURE_FILES {
        let path = format!(
            "{}/tests/fixtures/surface-rules-original-1.21.8/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        let bytes = std::fs::read(path).unwrap();
        assert_eq!(
            files[name],
            format!("{:x}", Sha256::digest(&bytes)),
            "{name}"
        );
    }
    assert_eq!(fixture.rows.len() + fixture.diagnostics, 38_891);
    assert_eq!(fixture.diagnostics, 2);
    assert_eq!(fixture.replay.len(), 38_889);
    assert_eq!(fixture.groups.len(), 80);
    assert_eq!(fixture.arrays.len(), 720);
    assert_eq!(fixture.steep.len(), 3);
    assert_eq!(fixture.manifest["round"], 3);
    assert!(fixture.manifest["invalid_evidence"]["paper_flat_bedrock"]
        .as_str()
        .unwrap()
        .starts_with("ungültig, nicht maßgeblich"));
    assert_eq!(
        fixture.manifest["source_manifest_sha256"],
        "15e4dbff186d8845143f72a0873f28bd35a87cbff4063dfe2e01d1e222969073"
    );
}

/// The Java `Double.toString` projection reproduces every original decimal token.
#[test]
fn java_double_text_reproduces_every_original_token() {
    let fixture = fixture();
    let mut checked = 0;
    let tokens = fixture
        .columns
        .iter()
        .flat_map(|c| c.split(';'))
        .chain(
            fixture
                .rows
                .iter()
                .map(|r| r.secondary.as_str())
                .filter(|s| *s != "-"),
        )
        .chain(CLAY.lines().skip(1).flat_map(|l| l.split('\t').skip(3)));
    for token in tokens {
        let (_, bits) = token.split_once('/').unwrap();
        let value = f64::from_bits(u64::from_str_radix(bits, 16).unwrap());
        assert_eq!(double_text(value), token);
        checked += 1;
    }
    assert!(checked > 10_000);
    for (value, text) in [
        (1.797_693_134_862_315_7e308, "1.7976931348623157E308"),
        (0.0125, "0.0125"),
        (-0.5, "-0.5"),
        (1e7, "1.0E7"),
        (1e-3, "0.001"),
        (9.0e-4, "9.0E-4"),
        (256.0, "256.0"),
    ] {
        assert_eq!(java_double(value), text);
    }
}
