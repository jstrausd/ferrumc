//! Shared GEN-05.10 fixture parsing, genuine producers, the trace recorder and
//! the mechanical projection into the fixture's tokens.
//!
//! Original expectations come only from the compact mechanical projection in
//! `tests/fixtures/surface-column-original-1.21.8` (Paper cells; Vanilla cells
//! are discriminating evidence only). Every file is checked against the
//! manifest digests before use, and states are classified by name, never by a
//! fixture flag (review condition A6). Policy, limit and contract-control
//! tests live in separate modules and never read original rows as expectations.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::sync::{Arc, Mutex, OnceLock};

use ferrumc_math::{BlockPos, ChunkPos};
use ferrumc_registry::biome::{BiomeParametersResourceId, BiomeResourceId};
use ferrumc_registry::block_state::{compute_state_id, state_id_to_block_name};
use ferrumc_registry::worldgen::NoiseSettingsResourceId;
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::super::compile::NodeKind;
use super::super::session::{Block, Column};
use super::super::tests::{bind, overworld_source, settings, temperatures, template};
use super::driver::{Observer, Phase};
use super::*;
use crate::biome_selection::{
    BiomeBuildBudget, BiomeQuartGeometry, BiomeSelectionSession, BiomeSourceSession, BiomeTree,
    BiomeWorkBudget, ChunkBiomeSamples, MAX_BIOME_BUILD_WORK, MAX_BIOME_SELECTION_WORK,
};
use crate::biome_zoom::BiomeZoomSemantics;
use crate::density::{
    ChunkMaterialSamples, NoiseChunkWorkBudget, RouterTemplate, SeededRouter, MAX_NOISE_CHUNK_WORK,
};
use crate::stored_biome::{
    StoredBiomeBuildBudget, StoredBiomeIdentity, SurfaceBiomeResolver, MAX_STORED_BIOME_BUILD_WORK,
};
use crate::surface_rules::{
    PreliminarySurfaceCorners, SeededSurfaceRules, SurfaceBlockInput, SurfaceColumnPos,
    SurfaceRuleOutcome, SurfaceRuleSession, SurfaceRuleTemplate, WorldSurfaceHeights,
    SURFACE_RULE_LEVEL_HEIGHT, SURFACE_RULE_LEVEL_MIN_Y,
};
use crate::{BlockStateId, WorldSeed};

mod allocation;
mod bits;
mod control;
mod refusals;
mod replay;
mod work;

const DIR: &str = "tests/fixtures/surface-column-original-1.21.8";
const MANIFEST: &str =
    include_str!("../../../tests/fixtures/surface-column-original-1.21.8/manifest.json");
const STATES: &str =
    include_str!("../../../tests/fixtures/surface-column-original-1.21.8/states.tsv");
const BIOMES: &str =
    include_str!("../../../tests/fixtures/surface-column-original-1.21.8/biomes.tsv");
const GROUPS: &str =
    include_str!("../../../tests/fixtures/surface-column-original-1.21.8/groups.tsv");
const CHUNKS: &str =
    include_str!("../../../tests/fixtures/surface-column-original-1.21.8/chunks.tsv");
const COLUMNS: &str =
    include_str!("../../../tests/fixtures/surface-column-original-1.21.8/columns.tsv");
const NOISES: &str =
    include_str!("../../../tests/fixtures/surface-column-original-1.21.8/noises.tsv");
const VANILLA: &str =
    include_str!("../../../tests/fixtures/surface-column-original-1.21.8/vanilla.tsv");

/// SHA-256 of the frozen `manifest.json` (GEN-05.10 freeze `ce970306…`).
const MANIFEST_SHA256: &str = "98fcf62a4fb0233b81b3c9c4e6a2529548e6c7841a32ad61b15354da00a6aecf";
/// The seven projected files with their embedded text.
const FILES: [(&str, &str); 7] = [
    ("biomes.tsv", BIOMES),
    ("chunks.tsv", CHUNKS),
    ("columns.tsv", COLUMNS),
    ("groups.tsv", GROUPS),
    ("noises.tsv", NOISES),
    ("states.tsv", STATES),
    ("vanilla.tsv", VANILLA),
];

pub(super) const MIN_Y: i32 = SURFACE_RULE_LEVEL_MIN_Y;
pub(super) const TOP_Y: i32 = MIN_Y + SURFACE_RULE_LEVEL_HEIGHT.cast_signed() - 1;
const LEVELS: usize = SURFACE_RULE_LEVEL_HEIGHT as usize;

/// Replay class of a group, as projected by the fixture's policy table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Class {
    Full,
    RefusedCentreOutsideDomain,
    RefusedInconsistentResolverSource,
}

pub(super) struct Group {
    pub gid: usize,
    pub id: String,
    pub settings: NoiseSettingsResourceId,
    pub seed: i64,
    pub center: ChunkPos,
    pub class: Class,
    pub corners: PreliminarySurfaceCorners,
    pub sea_level: i32,
    pub digests: Vec<String>,
}

pub(super) struct ChunkRow {
    pub s0_heights: String,
    pub s0_post: String,
    pub s0_rle: String,
    pub final_heights: String,
    pub final_post: String,
    pub final_sha: String,
}

/// One `columns.tsv` row; `cells` follow [`TRACE_CELLS`].
pub(super) struct ColumnRow {
    pub lx: u8,
    pub lz: u8,
    pub biome: usize,
    pub cells: [String; 9],
}

/// Cell order of [`ColumnRow::cells`] (the trace tokens compared exactly).
pub(super) const TRACE_CELLS: [&str; 9] = [
    "height_plus_1",
    "eroded_writes",
    "first_call_height_plus_1",
    "surface_depth",
    "call_runs",
    "call_outcomes",
    "steep",
    "min_surface",
    "frozen_writes",
];

pub(super) struct NoiseRow {
    pub gid: usize,
    pub lx: u8,
    pub lz: u8,
    pub paper_bits: Vec<u64>,
    pub vanilla_bits: Vec<u64>,
    pub ints: String,
    pub paper_doubles: String,
    pub vanilla_doubles: String,
    pub melt: bool,
}

pub(super) struct Fixture {
    pub manifest: Value,
    /// Raw ids in fixture index order (identity checked by name).
    pub raw: Vec<u32>,
    pub index_of_raw: HashMap<u32, usize>,
    pub biomes: Vec<BiomeResourceId>,
    pub groups: Vec<Group>,
    pub chunks: Vec<ChunkRow>,
    pub columns: Vec<Vec<ColumnRow>>,
    pub noises: Vec<NoiseRow>,
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

/// `minecraft:name[key=value,…]` → registry state id (identity by name, A6).
fn state_id_of(text: &str) -> u32 {
    let (name, properties) = match text.split_once('[') {
        Some((name, rest)) => (name, rest.strip_suffix(']').unwrap()),
        None => (text, ""),
    };
    let mut map = std::collections::BTreeMap::new();
    for pair in properties.split(',').filter(|p| !p.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap();
        map.insert(key, value);
    }
    compute_state_id(name, &map).unwrap()
}

pub(super) fn sha(text: &[u8]) -> String {
    format!("{:x}", Sha256::digest(text))
}

#[allow(clippy::too_many_lines)] // one linear pass over the eight fixture files
fn parse() -> Fixture {
    // A6: the pinned manifest and every file digest, before any use.
    assert_eq!(sha(MANIFEST.as_bytes()), MANIFEST_SHA256);
    let manifest: Value = serde_json::from_str(MANIFEST).unwrap();
    let files = manifest["files"].as_object().unwrap();
    assert_eq!(files.len(), FILES.len());
    for (name, text) in FILES {
        assert_eq!(files[name], sha(text.as_bytes()), "{name}");
        let path = format!("{}/{DIR}/{name}", env!("CARGO_MANIFEST_DIR"));
        assert_eq!(std::fs::read(path).unwrap(), text.as_bytes(), "{name}");
    }
    let mut raw = Vec::new();
    for (i, r) in table(STATES, 10).enumerate() {
        assert_eq!(r[0].parse::<usize>().unwrap(), i);
        let id: u32 = r[2].parse().unwrap();
        // Identity from the name through the registry; no flag column is read.
        assert_eq!(state_id_of(r[1]), id, "{}", r[1]);
        raw.push(id);
    }
    let index_of_raw = raw.iter().enumerate().map(|(i, r)| (*r, i)).collect();
    let biomes = table(BIOMES, 2)
        .map(|r| BiomeResourceId::from_name(r[1]).unwrap())
        .collect();
    let groups: Vec<Group> = table(GROUPS, 13)
        .enumerate()
        .map(|(gid, r)| {
            let corners: Vec<i32> = r[8].split(',').map(|v| v.parse().unwrap()).collect();
            Group {
                gid,
                id: r[0].to_owned(),
                settings: NoiseSettingsResourceId::from_name(r[2]).unwrap(),
                seed: r[3].parse().unwrap(),
                center: ChunkPos::new(r[4].parse().unwrap(), r[5].parse().unwrap()),
                class: match r[7] {
                    "full" => Class::Full,
                    "refused:CentreOutsideDomain" => Class::RefusedCentreOutsideDomain,
                    "refused:InconsistentResolver(Source)" => {
                        Class::RefusedInconsistentResolverSource
                    }
                    other => panic!("unknown class {other}"),
                },
                corners: PreliminarySurfaceCorners::new(
                    corners[0], corners[1], corners[2], corners[3],
                ),
                sea_level: r[9].parse().unwrap(),
                digests: r[11].split(',').map(str::to_owned).collect(),
            }
        })
        .collect();
    let gid_of: HashMap<&str, usize> = groups.iter().map(|g| (g.id.as_str(), g.gid)).collect();
    let chunks: Vec<ChunkRow> = table(CHUNKS, 8)
        .enumerate()
        .map(|(i, r)| {
            assert_eq!(gid_of[r[0]], i);
            ChunkRow {
                s0_heights: r[1].to_owned(),
                s0_post: r[2].to_owned(),
                s0_rle: r[3].to_owned(),
                final_heights: r[4].to_owned(),
                final_post: r[6].to_owned(),
                final_sha: r[7].to_owned(),
            }
        })
        .collect();
    let mut columns: Vec<Vec<ColumnRow>> = groups.iter().map(|_| Vec::new()).collect();
    for r in table(COLUMNS, 13) {
        let rows = &mut columns[gid_of[r[0]]];
        let (lx, lz): (u8, u8) = (r[1].parse().unwrap(), r[2].parse().unwrap());
        assert_eq!(usize::from(lx) * 16 + usize::from(lz), rows.len());
        rows.push(ColumnRow {
            lx,
            lz,
            biome: r[3].parse().unwrap(),
            cells: std::array::from_fn(|i| r[4 + i].to_owned()),
        });
    }
    let hex = |cell: &str| -> Vec<u64> {
        cell.split(',')
            .map(|h| u64::from_str_radix(h, 16).unwrap())
            .collect()
    };
    let noises = table(NOISES, 9)
        .map(|r| NoiseRow {
            gid: gid_of[r[0]],
            lx: r[1].parse().unwrap(),
            lz: r[2].parse().unwrap(),
            paper_bits: hex(r[3]),
            vanilla_bits: hex(r[4]),
            ints: r[5].to_owned(),
            paper_doubles: r[6].to_owned(),
            vanilla_doubles: r[7].to_owned(),
            melt: match r[8] {
                "0" => false,
                "1" => true,
                other => panic!("melt {other}"),
            },
        })
        .collect();
    assert_eq!(table(VANILLA, 5).count(), groups.len());
    Fixture {
        manifest,
        raw,
        index_of_raw,
        biomes,
        groups,
        chunks,
        columns,
        noises,
    }
}

pub(super) fn fixture() -> &'static Fixture {
    static FIXTURE: OnceLock<Fixture> = OnceLock::new();
    FIXTURE.get_or_init(parse)
}

/// Run-length tokens `value*count` joined by `.` (fixture RLE).
pub(super) fn unrle(text: &str) -> Vec<usize> {
    let mut out = Vec::with_capacity(LEVELS);
    for run in text.split('.') {
        let (value, count) = run.split_once('*').unwrap();
        let value: usize = value.parse().unwrap();
        out.extend(std::iter::repeat_n(value, count.parse().unwrap()));
    }
    out
}

fn rle(values: &[String]) -> String {
    if values.is_empty() {
        return "-".to_owned();
    }
    let mut out = Vec::new();
    let mut i = 0;
    while i < values.len() {
        let mut j = i;
        while j < values.len() && values[j] == values[i] {
            j += 1;
        }
        out.push(format!("{}*{}", values[i], j - i));
        i = j;
    }
    out.join(".")
}

/// Material-order index `((y + 64) · 16 + lx) · 16 + lz`.
pub(super) const fn index(lx: usize, y: i32, lz: usize) -> usize {
    ((y - MIN_Y) as usize * 16 + lx) * 16 + lz
}

/// The positions of a `sec:packed,…;…` post-processing token as material indices.
pub(super) fn post_positions(token: &str) -> HashSet<usize> {
    let mut out = HashSet::new();
    if token == "-" {
        return out;
    }
    for part in token.split(';') {
        let (section, values) = part.split_once(':').unwrap();
        let section: usize = section.parse().unwrap();
        for value in values.split(',').filter(|v| !v.is_empty()) {
            let packed: usize = value.parse().unwrap();
            let (lx, ly, lz) = (packed & 15, (packed >> 4) & 15, (packed >> 8) & 15);
            assert!(out.insert(((section * 16 + ly) * 16 + lx) * 16 + lz));
        }
    }
    out
}

/// Material indices of every set bit of a fluid bitmap.
pub(super) fn bitmap_positions(words: &[u64]) -> HashSet<usize> {
    (0..words.len() * 64)
        .filter(|i| words[i / 64] & (1 << (i % 64)) != 0)
        .collect()
}

/// Genuine GEN-05.3/.4/.5 outputs of one admitted group.
pub(super) struct Produced {
    pub gid: usize,
    pub material: ChunkMaterialSamples,
    pub corners: PreliminarySurfaceCorners,
    pub identity: StoredBiomeIdentity,
    pub arrays: Vec<ChunkBiomeSamples>,
}

/// A complete Paper-semantics GEN-05.8 resolver over nine arrays.
pub(super) fn resolver_of<'a>(
    identity: StoredBiomeIdentity,
    center: ChunkPos,
    arrays: &'a [ChunkBiomeSamples],
) -> SurfaceBiomeResolver<'a> {
    let refs: Vec<&'a ChunkBiomeSamples> = arrays.iter().collect();
    SurfaceBiomeResolver::new(
        identity,
        center,
        &refs,
        BiomeZoomSemantics::Paper1_21_8Build60,
        StoredBiomeBuildBudget::new(MAX_STORED_BIOME_BUILD_WORK).unwrap(),
    )
    .unwrap()
}

impl Produced {
    pub fn resolver(&self) -> SurfaceBiomeResolver<'_> {
        resolver_of(self.identity, self.material.chunk_pos(), &self.arrays)
    }
}

pub(super) fn level_geometry() -> BiomeQuartGeometry {
    BiomeQuartGeometry::new(SURFACE_RULE_LEVEL_MIN_Y, SURFACE_RULE_LEVEL_HEIGHT).unwrap()
}

pub(super) fn noise_budget() -> NoiseChunkWorkBudget {
    NoiseChunkWorkBudget::new(MAX_NOISE_CHUNK_WORK).unwrap()
}

type Shared<K, V> = OnceLock<Mutex<Vec<(K, &'static V)>>>;

/// Seeded routers per (settings, seed), built once.
pub(super) fn router(settings: NoiseSettingsResourceId, seed: i64) -> &'static SeededRouter {
    static ROUTERS: Shared<(NoiseSettingsResourceId, i64), SeededRouter> = OnceLock::new();
    let mut all = ROUTERS.get_or_init(Default::default).lock().unwrap();
    if let Some((_, r)) = all.iter().find(|(k, _)| *k == (settings, seed)) {
        return r;
    }
    let router: &'static SeededRouter = Box::leak(Box::new(
        RouterTemplate::compile_builtin(settings)
            .unwrap()
            .bind_seed(WorldSeed::new(seed))
            .unwrap(),
    ));
    all.push(((settings, seed), router));
    router
}

pub(super) fn biome_tree() -> Arc<BiomeTree> {
    static TREE: OnceLock<Arc<BiomeTree>> = OnceLock::new();
    Arc::clone(TREE.get_or_init(|| {
        Arc::new(
            BiomeTree::from_builtin(
                BiomeParametersResourceId::from_name("minecraft:overworld").unwrap(),
                BiomeBuildBudget::new(MAX_BIOME_BUILD_WORK).unwrap(),
            )
            .unwrap(),
        )
    }))
}

/// GEN-05.4 material and GEN-05.3 preliminary corners of one chunk.
pub(super) fn material(
    settings: NoiseSettingsResourceId,
    seed: i64,
    chunk: ChunkPos,
) -> (ChunkMaterialSamples, PreliminarySurfaceCorners) {
    let router = router(settings, seed);
    let material = router
        .fresh_paper_noise_chunk(chunk, noise_budget())
        .unwrap()
        .sample_chunk_materials()
        .unwrap();
    let mut session = router
        .fresh_paper_noise_chunk(chunk, noise_budget())
        .unwrap();
    let mut corner = |dx: i32, dz: i32| {
        session
            .preliminary_surface_level(BlockPos::new(
                chunk.x().wrapping_add(dx).wrapping_mul(16),
                0,
                chunk.z().wrapping_add(dz).wrapping_mul(16),
            ))
            .unwrap()
    };
    let corners =
        PreliminarySurfaceCorners::new(corner(0, 0), corner(1, 0), corner(0, 1), corner(1, 1));
    (material, corners)
}

/// GEN-05.5 multinoise arrays of the 3×3 around `center` (cold history, row-major).
pub(super) fn arrays(
    settings: NoiseSettingsResourceId,
    seed: i64,
    center: ChunkPos,
) -> Vec<ChunkBiomeSamples> {
    let router = router(settings, seed);
    let mut session = BiomeSourceSession::from_multinoise(BiomeSelectionSession::new(
        biome_tree(),
        BiomeWorkBudget::new(MAX_BIOME_SELECTION_WORK).unwrap(),
    ));
    let mut out = Vec::with_capacity(9);
    for dx in -1..=1 {
        for dz in -1..=1 {
            let chunk = ChunkPos::new(center.x() + dx, center.z() + dz);
            let mut noise = router
                .fresh_paper_noise_chunk(chunk, noise_budget())
                .unwrap();
            out.push(
                session
                    .sample_chunk_biomes(&mut noise, level_geometry())
                    .unwrap(),
            );
        }
    }
    out
}

pub(super) fn identity(settings: NoiseSettingsResourceId, seed: i64) -> StoredBiomeIdentity {
    StoredBiomeIdentity::new(
        WorldSeed::new(seed),
        settings,
        overworld_source(),
        level_geometry(),
    )
}

pub(super) fn produce(
    settings: NoiseSettingsResourceId,
    seed: i64,
    center: ChunkPos,
    gid: usize,
) -> Produced {
    let (material, corners) = material(settings, seed, center);
    Produced {
        gid,
        material,
        corners,
        identity: identity(settings, seed),
        arrays: arrays(settings, seed, center),
    }
}

/// Genuine producer outputs of every `full` group, built once and in parallel.
pub(super) fn produced() -> &'static [Produced] {
    static PRODUCED: OnceLock<Vec<Produced>> = OnceLock::new();
    PRODUCED.get_or_init(|| {
        let full: Vec<&Group> = fixture()
            .groups
            .iter()
            .filter(|g| g.class == Class::Full)
            .collect();
        let threads = std::thread::available_parallelism().map_or(2, |n| n.get().min(4));
        let mut out: Vec<Produced> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..threads)
                .map(|t| {
                    let mine: Vec<&Group> = full.iter().skip(t).step_by(threads).copied().collect();
                    scope.spawn(move || {
                        mine.into_iter()
                            .map(|g| produce(g.settings, g.seed, g.center, g.gid))
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            handles
                .into_iter()
                .flat_map(|h| h.join().unwrap())
                .collect()
        });
        out.sort_by_key(|p| p.gid);
        out
    })
}

/// The producer outputs of a `full` group by its id.
pub(super) fn produced_group(id: &str) -> (&'static Group, &'static Produced) {
    let group = fixture().groups.iter().find(|g| g.id == id).unwrap();
    let produced = produced().iter().find(|p| p.gid == group.gid).unwrap();
    (group, produced)
}

pub(super) fn bind_budget(units: u32) -> SurfaceColumnBindBudget {
    SurfaceColumnBindBudget::new(units).unwrap()
}

pub(super) fn work_budget(units: u32) -> SurfaceColumnWorkBudget {
    SurfaceColumnWorkBudget::new(units).unwrap()
}

/// Rules and column bindings per (settings, seed), built once.
pub(super) fn bindings(
    settings: NoiseSettingsResourceId,
    seed: i64,
) -> &'static (SeededSurfaceRules, SeededSurfaceColumns) {
    static BINDINGS: Shared<
        (NoiseSettingsResourceId, i64),
        (SeededSurfaceRules, SeededSurfaceColumns),
    > = OnceLock::new();
    let mut all = BINDINGS.get_or_init(Default::default).lock().unwrap();
    if let Some((_, b)) = all.iter().find(|(k, _)| *k == (settings, seed)) {
        return b;
    }
    let rules = bind(template(settings), seed);
    let columns =
        SeededSurfaceColumns::bind(&rules, bind_budget(MAX_SURFACE_COLUMN_BIND_WORK)).unwrap();
    let entry: &'static (SeededSurfaceRules, SeededSurfaceColumns) =
        Box::leak(Box::new((rules, columns)));
    all.push(((settings, seed), entry));
    entry
}

pub(super) fn overworld_settings() -> NoiseSettingsResourceId {
    settings("minecraft:overworld")
}

/// One recorded column in driver terms.
#[derive(Debug, Default, Clone)]
pub(super) struct ColumnTrace {
    pub lx: u8,
    pub lz: u8,
    pub biome: Option<BiomeResourceId>,
    pub height_plus_1: i32,
    pub eroded: Vec<(i32, BlockStateId)>,
    pub first_call_height_plus_1: i32,
    pub surface_depth: i32,
    pub calls: Vec<(SurfaceBlockInput, SurfaceRuleOutcome)>,
    pub steep: Option<(usize, bool, [i32; 4])>,
    pub min_surface: Option<i32>,
    pub frozen: Vec<(i32, BlockStateId)>,
    pub work: u32,
}

/// Global event order of one driven chunk (writes and first steep reads).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Event {
    Write {
        lx: u8,
        lz: u8,
        y: i32,
        state: BlockStateId,
    },
    Steep {
        lx: u8,
        lz: u8,
        read: [i32; 4],
    },
}

/// Recording observer: the per-column trace, the event log and the RR-H2
/// per-call work model check.
pub(super) struct Recorder<'r> {
    rules: &'r SeededSurfaceRules,
    pub columns: Vec<ColumnTrace>,
    pub events: Vec<Event>,
    current: ColumnTrace,
    before: Option<(Option<Column>, u32)>,
    /// Calls that consulted / did not consult the resolver memo (RR-H2).
    pub biome_called: usize,
    pub biome_not_called: usize,
    pub max_call_work: u32,
}

impl<'r> Recorder<'r> {
    pub fn new(rules: &'r SeededSurfaceRules) -> Self {
        Self {
            rules,
            columns: Vec::with_capacity(256),
            events: Vec::new(),
            current: ColumnTrace::default(),
            before: None,
            biome_called: 0,
            biome_not_called: 0,
            max_call_work: 0,
        }
    }
}

impl Recorder<'_> {
    /// Calls of a column whose end was not observed (synthetic controls).
    pub fn current_calls(&self) -> &[(SurfaceBlockInput, SurfaceRuleOutcome)] {
        &self.current.calls
    }
}

impl Observer for Recorder<'_> {
    fn column(&mut self, local_x: u8, local_z: u8, biome: BiomeResourceId, height_plus_1: i32) {
        self.current = ColumnTrace {
            lx: local_x,
            lz: local_z,
            biome: Some(biome),
            height_plus_1,
            ..ColumnTrace::default()
        };
    }
    fn write(&mut self, phase: Phase, y: i32, state: BlockStateId) {
        match phase {
            Phase::Eroded => self.current.eroded.push((y, state)),
            Phase::Frozen => self.current.frozen.push((y, state)),
            Phase::Rule => {}
        }
        self.events.push(Event::Write {
            lx: self.current.lx,
            lz: self.current.lz,
            y,
            state,
        });
    }
    fn loop_start(&mut self, height_plus_1: i32, session: &SurfaceRuleSession<'_>) {
        self.current.first_call_height_plus_1 = height_plus_1;
        self.current.surface_depth = session.surface_depth().unwrap();
    }
    fn before_evaluate(&mut self, session: &SurfaceRuleSession<'_>) {
        self.before = Some((session.column, session.work_used()));
    }
    fn evaluated(
        &mut self,
        input: SurfaceBlockInput,
        outcome: SurfaceRuleOutcome,
        session: &SurfaceRuleSession<'_>,
    ) {
        let (before, used) = self.before.take().unwrap();
        let before = before.unwrap();
        let after = session.column.unwrap();
        if before.steep.is_none() {
            if let Some((steep, read)) = after.steep {
                self.current.steep = Some((self.current.calls.len(), steep, read));
                self.events.push(Event::Steep {
                    lx: self.current.lx,
                    lz: self.current.lz,
                    read,
                });
            }
        }
        // RR-H2: the session's own units must equal an independent model in
        // which the resolver is charged iff a biome node or the temperature
        // singleton is evaluated by the preorder walk.
        let spent = session.work_used() - used;
        let (model, consulted) = call_work(
            self.rules.template(),
            before,
            after,
            session.block,
            input.y(),
        );
        assert_eq!(
            spent,
            model,
            "call work at y {} of column {:?}",
            input.y(),
            after.pos
        );
        if consulted {
            self.biome_called += 1;
        } else {
            self.biome_not_called += 1;
        }
        self.max_call_work = self.max_call_work.max(spent);
        self.current.calls.push((input, outcome));
    }
    fn column_end(&mut self, session: &SurfaceRuleSession<'_>, column_work: u32) {
        self.current.min_surface = session.column.unwrap().min_surface;
        self.current.work = column_work;
        self.columns.push(std::mem::take(&mut self.current));
    }
}

/// Independent per-call work model of `SurfaceRuleSession::evaluate` (RR-H2).
///
/// Walks the compiled preorder tree with the truth values the call left in
/// the session (per-block bits, column caches) and charges: 1 for the root and
/// per pushed child, 1 per tested node of a `not` chain, 1 per fresh noise
/// threshold, per gradient draw strictly inside its band, per band sample, per
/// first secondary noise / minimum surface of the column, per temperature, and
/// exactly one resolver consultation iff a `biome` node or the `temperature`
/// singleton is evaluated. Returns `(units, resolver consulted)`.
#[allow(clippy::too_many_lines)] // one self-contained model of the evaluator
pub(super) fn call_work(
    template: &SurfaceRuleTemplate,
    before: Column,
    after: Column,
    block: Block,
    y: i32,
) -> (u32, bool) {
    #[allow(clippy::struct_excessive_bools)] // the four per-call memo flags
    struct Walk<'t> {
        template: &'t SurfaceRuleTemplate,
        before: Column,
        after: Column,
        block: Block,
        y: i32,
        units: u32,
        biome: bool,
        temperature: bool,
        secondary: bool,
        minimum: bool,
    }
    impl Walk<'_> {
        fn rule(&mut self, node: usize) -> bool {
            match self.template.nodes[node].kind {
                NodeKind::Sequence { first, len } => {
                    for i in 0..len {
                        self.units += 1;
                        let child = usize::from(self.template.children[usize::from(first + i)]);
                        if self.rule(child) {
                            return true;
                        }
                    }
                    false
                }
                NodeKind::Condition { test, then } => {
                    if self.test(usize::from(test)) {
                        self.units += 1;
                        self.rule(usize::from(then))
                    } else {
                        false
                    }
                }
                NodeKind::Block(_) => true,
                NodeKind::Bandlands => {
                    self.units += 1;
                    true
                }
                other => panic!("rule node {other:?}"),
            }
        }
        fn test(&mut self, mut node: usize) -> bool {
            let mut negate = false;
            loop {
                self.units += 1;
                match self.template.nodes[node].kind {
                    NodeKind::Not(target) => {
                        negate = !negate;
                        node = usize::from(target);
                    }
                    kind => return self.leaf(node, kind) != negate,
                }
            }
        }
        fn per_block(&self, node: usize) -> bool {
            assert!(
                self.block.done.get(node),
                "per-block node {node} not computed"
            );
            self.block.value.get(node)
        }
        fn consult(&mut self) {
            if !self.biome {
                self.biome = true;
                self.units += 1;
            }
        }
        fn leaf(&mut self, node: usize, kind: NodeKind) -> bool {
            match kind {
                NodeKind::Biome { .. } => {
                    self.consult();
                    self.per_block(node)
                }
                NodeKind::NoiseThreshold(_) => {
                    if !self.before.noise_done.get(node) {
                        self.units += 1;
                    }
                    self.after.noise_value.get(node)
                }
                NodeKind::VerticalGradient(index) | NodeKind::PaperFlatBedrock(index) => {
                    let gradient = self.template.gradients[usize::from(index)];
                    if self.y > gradient.true_y && self.y < gradient.false_y {
                        self.units += 1;
                    }
                    self.per_block(node)
                }
                NodeKind::YAbove(_) | NodeKind::Water(_) => self.per_block(node),
                NodeKind::StoneDepth(index) => {
                    let check = self.template.stone_checks[usize::from(index)];
                    if check.range != 0 && self.before.secondary.is_none() && !self.secondary {
                        self.secondary = true;
                        self.units += 1;
                    }
                    self.per_block(node)
                }
                NodeKind::Steep => self.after.steep.unwrap().0,
                NodeKind::Hole => self.after.hole.unwrap(),
                NodeKind::AbovePreliminarySurface => {
                    if self.before.min_surface.is_none() && !self.minimum {
                        self.minimum = true;
                        self.units += 1;
                    }
                    self.y >= self.after.min_surface.unwrap()
                }
                NodeKind::Temperature => {
                    if !self.temperature {
                        self.temperature = true;
                        self.consult();
                        self.units += 1;
                    }
                    self.block.temperature.unwrap()
                }
                other => panic!("leaf {other:?}"),
            }
        }
    }
    let mut walk = Walk {
        template,
        before,
        after,
        block,
        y,
        units: 1,
        biome: false,
        temperature: false,
        secondary: false,
        minimum: false,
    };
    walk.rule(0);
    (walk.units, walk.biome)
}

/// Fixture state index of a runtime state.
pub(super) fn state_index(state: BlockStateId) -> usize {
    fixture().index_of_raw[&state.as_u32()]
}

/// `ytop:count:state` runs of descending writes, `-` when none.
pub(super) fn writes_text(writes: &[(i32, BlockStateId)]) -> String {
    let mut out = Vec::new();
    let mut at = 0;
    while at < writes.len() {
        let (y, state) = writes[at];
        let mut run = 1;
        while at + run < writes.len()
            && writes[at + run] == (y - i32::try_from(run).unwrap(), state)
        {
            run += 1;
        }
        out.push(format!("{y}:{run}:{}", state_index(state)));
        at += run;
    }
    if out.is_empty() {
        "-".to_owned()
    } else {
        out.join(";")
    }
}

/// The oracle's `runs()` grouping of `(y, above, below, water)` call inputs.
fn call_runs(calls: &[(SurfaceBlockInput, SurfaceRuleOutcome)]) -> String {
    let key = |input: SurfaceBlockInput| {
        (
            input.y(),
            input.stone_depth_above(),
            input.stone_depth_below(),
            input.water_height(),
        )
    };
    let mut out = Vec::new();
    let mut at = 0;
    while at < calls.len() {
        let first = key(calls[at].0);
        let mut run = 1;
        while at + run < calls.len() {
            let next = key(calls[at + run].0);
            let step = i32::try_from(run).unwrap();
            if next.0 != first.0 - step
                || next.1 != first.1 + step
                || next.2 != first.2 - step
                || next.3 != first.3
            {
                break;
            }
            run += 1;
        }
        let water = first.3.map_or_else(|| "-".to_owned(), |w| w.to_string());
        out.push(format!("{}:{}:{}:{water}:{run}", first.0, first.1, first.2));
        at += run;
    }
    if out.is_empty() {
        "-".to_owned()
    } else {
        out.join(";")
    }
}

/// Projects a recorded column into the fixture's nine trace tokens.
pub(super) fn project(trace: &ColumnTrace) -> [String; 9] {
    let outcomes: Vec<String> = trace
        .calls
        .iter()
        .map(|(_, outcome)| match outcome {
            SurfaceRuleOutcome::NoRule => "n".to_owned(),
            SurfaceRuleOutcome::State(state) => state_index(*state).to_string(),
        })
        .collect();
    let steep = trace.steep.map_or_else(
        || "-".to_owned(),
        |(call, steep, read)| {
            let mut text = format!("{call}:{}:", u8::from(steep));
            for (i, h) in read.iter().enumerate() {
                if i > 0 {
                    text.push(',');
                }
                write!(text, "{h}").unwrap();
            }
            text
        },
    );
    [
        trace.height_plus_1.to_string(),
        writes_text(&trace.eroded),
        trace.first_call_height_plus_1.to_string(),
        trace.surface_depth.to_string(),
        call_runs(&trace.calls),
        rle(&outcomes),
        steep,
        trace
            .min_surface
            .map_or_else(|| "-".to_owned(), |m| m.to_string()),
        writes_text(&trace.frozen),
    ]
}

/// Original air blocks by name (D1); the test never reads a fixture flag.
pub(super) fn is_air(raw: u32) -> bool {
    matches!(
        state_id_to_block_name(raw),
        Some("air" | "cave_air" | "void_air")
    )
}

/// One first steep read: the column, the four heights the session read, the
/// live view at that moment and the post-NOISE view.
pub(super) struct SteepRead {
    pub column: (u8, u8),
    pub read: [i32; 4],
    pub live: [i32; 4],
    pub initial: [i32; 4],
}

/// Independent live `WORLD_SURFACE_WG` model over raw state ids, updated with
/// the original `Heightmap.update` rule.
pub(super) struct LiveModel {
    pub states: Vec<u32>,
    pub heights: [[i32; 16]; 16],
}

impl LiveModel {
    pub fn new(states: Vec<u32>) -> Self {
        let mut model = Self {
            states,
            heights: [[MIN_Y - 1; 16]; 16],
        };
        for lx in 0..16 {
            for lz in 0..16 {
                model.heights[lx][lz] = model.top(lx, TOP_Y, lz);
            }
        }
        model
    }
    fn top(&self, lx: usize, from: i32, lz: usize) -> i32 {
        (MIN_Y..=from)
            .rev()
            .find(|y| !is_air(self.states[index(lx, *y, lz)]))
            .unwrap_or(MIN_Y - 1)
    }
    /// Applies one write; returns whether the column's height changed.
    pub fn write(&mut self, lx: usize, y: i32, lz: usize, state: u32) -> bool {
        self.states[index(lx, y, lz)] = state;
        let top = self.heights[lx][lz];
        let new = if !is_air(state) {
            top.max(y)
        } else if y == top {
            self.top(lx, y - 1, lz)
        } else {
            top
        };
        self.heights[lx][lz] = new;
        new != top
    }
    /// The four steep reads `[lz−1, lz+1, lx−1, lx+1]`, clamped to the chunk.
    pub fn steep_view(&self, lx: usize, lz: usize) -> [i32; 4] {
        [
            self.heights[lx][lz.saturating_sub(1)],
            self.heights[lx][(lz + 1).min(15)],
            self.heights[lx.saturating_sub(1)][lz],
            self.heights[(lx + 1).min(15)][lz],
        ]
    }
    /// Replays an event log; returns every steep event with its live and
    /// post-NOISE views and asserts nothing.
    pub fn replay(&mut self, events: &[Event]) -> Vec<SteepRead> {
        let initial = Self::new(self.states.clone());
        let mut out = Vec::new();
        for event in events {
            match *event {
                Event::Write { lx, lz, y, state } => {
                    self.write(usize::from(lx), y, usize::from(lz), state.as_u32());
                }
                Event::Steep { lx, lz, read } => {
                    let (x, z) = (usize::from(lx), usize::from(lz));
                    out.push(SteepRead {
                        column: (lx, lz),
                        read,
                        live: self.steep_view(x, z),
                        initial: initial.steep_view(x, z),
                    });
                }
            }
        }
        out
    }
}

/// Drives one chunk with the recorder through the full refusal order.
pub(super) fn drive_recorded<'r>(
    columns: &SeededSurfaceColumns,
    rules: &'r SeededSurfaceRules,
    resolver: &SurfaceBiomeResolver<'_>,
    corners: PreliminarySurfaceCorners,
    material: &ChunkMaterialSamples,
    buffer: &mut SurfaceChunkBuffer,
) -> (SurfaceChunkReport, Recorder<'r>) {
    let mut recorder = Recorder::new(rules);
    let report = columns
        .build_observed(
            SurfaceChunkInput::new(rules, resolver, temperatures(), corners, material),
            work_budget(SURFACE_CHUNK_WORK),
            buffer,
            &mut recorder,
        )
        .unwrap();
    (report, recorder)
}

/// SHA-256 of the comma-joined raw ids in material order (fixture digest).
pub(super) fn states_digest(states: &[BlockStateId]) -> String {
    let mut text = String::with_capacity(states.len() * 6);
    for (i, state) in states.iter().enumerate() {
        if i > 0 {
            text.push(',');
        }
        write!(text, "{}", state.as_u32()).unwrap();
    }
    sha(text.as_bytes())
}

/// Heights `[lx][lz]` in fixture order (`k = lx·16 + lz`), comma-joined.
pub(super) fn heights_text(heights: &WorldSurfaceHeights) -> String {
    let mut out = Vec::with_capacity(256);
    for lx in 0..16 {
        for lz in 0..16 {
            out.push(heights.height(lx, lz).unwrap().to_string());
        }
    }
    out.join(",")
}

/// A column position of `chunk`.
pub(super) fn column_pos(chunk: ChunkPos, lx: u8, lz: u8) -> SurfaceColumnPos {
    SurfaceColumnPos::new(chunk, lx, lz).unwrap()
}
