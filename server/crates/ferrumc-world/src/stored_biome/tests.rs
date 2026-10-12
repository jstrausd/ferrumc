//! Shared fixture parsing, genuine GEN-05.5 producers and private controls.
//!
//! Original expectations come only from the compact mechanical projection in
//! `tests/fixtures/stored-biome-original-1.21.8`. Synthetic controls implement
//! the private column view and reach only the checked kernel; every public
//! build/query test borrows actual GEN-05.5 `ChunkBiomeSamples`.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use super::*;
use crate::biome_selection::{
    BiomeBuildBudget, BiomeSelectionSession, BiomeSourceSession, BiomeTree, BiomeWorkBudget,
    MAX_BIOME_BUILD_WORK, MAX_BIOME_SELECTION_WORK,
};
use crate::density::{NoiseChunkWorkBudget, RouterTemplate, SeededRouter, MAX_NOISE_CHUNK_WORK};
use ferrumc_registry::biome::BiomeParametersResourceId;
use serde_json::Value;

mod admission;
mod allocation;
mod halo_order;
mod oracle;

const MANIFEST: &str =
    include_str!("../../tests/fixtures/stored-biome-original-1.21.8/manifest.json");
const QUERIES: &str = include_str!("../../tests/fixtures/stored-biome-original-1.21.8/queries.tsv");
const ARRAYS: &str = include_str!("../../tests/fixtures/stored-biome-original-1.21.8/arrays.tsv");
const HISTORIES: &str =
    include_str!("../../tests/fixtures/stored-biome-original-1.21.8/histories.tsv");

pub(super) const SEMANTICS: [BiomeZoomSemantics; 2] = [
    BiomeZoomSemantics::Vanilla1_21_8,
    BiomeZoomSemantics::Paper1_21_8Build60,
];

pub(super) struct Group {
    pub id: String,
    pub axis: bool,
    pub center: ChunkPos,
}

pub(super) struct Seed {
    pub world: bool,
    pub value: i64,
    pub zoom_bits: i64,
}

impl Seed {
    pub fn zoom_seed(&self) -> BiomeZoomSeed {
        if self.world {
            BiomeZoomSeed::from_world_seed(WorldSeed::new(self.value))
        } else {
            BiomeZoomSeed::from_obfuscated_bits(self.value)
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct Query {
    pub group: usize,
    pub kind: u8,
    pub seed: usize,
    pub input: [i32; 3],
    pub quart: StoredBiomeQuartPos,
    pub chunk: ChunkPos,
    /// Original resource, or `None` for a controlled hook (policy) row.
    pub value: Option<BiomeResourceId>,
}

impl Query {
    pub const fn block(&self) -> BlockPos {
        BlockPos::new(self.input[0], self.input[1], self.input[2])
    }
}

pub(super) struct ArrayRow {
    pub group: usize,
    pub chunk: ChunkPos,
    pub geometry: BiomeQuartGeometry,
    pub values: Vec<BiomeResourceId>,
}

pub(super) struct HistoryRow {
    pub group: usize,
    pub sequence: usize,
    pub chunk: ChunkPos,
    pub history: String,
    pub settings: NoiseSettingsResourceId,
    pub source: String,
    pub geometry: BiomeQuartGeometry,
    pub initial: Option<(usize, BiomeResourceId)>,
    pub last: Option<(usize, BiomeResourceId)>,
}

pub(super) struct Fixture {
    pub groups: Vec<Group>,
    pub seeds: Vec<Seed>,
    pub queries: Vec<Query>,
    pub arrays: Vec<ArrayRow>,
    pub histories: Vec<HistoryRow>,
    pub evidence: Vec<String>,
    pub manifest: Value,
}

fn rows(text: &str, columns: usize) -> impl Iterator<Item = Vec<&str>> {
    let mut lines = text.lines();
    let header = lines.next().unwrap();
    assert_eq!(header.split('\t').count(), columns);
    lines.map(move |line| {
        let row: Vec<_> = line.split('\t').collect();
        assert_eq!(row.len(), columns, "{line}");
        row
    })
}

fn chunk(x: &str, z: &str) -> ChunkPos {
    ChunkPos::new(x.parse().unwrap(), z.parse().unwrap())
}

fn optional_point(
    ordinal: &str,
    biome: &str,
    roster: &[BiomeResourceId],
) -> Option<(usize, BiomeResourceId)> {
    match (ordinal, biome) {
        ("-", "-") => None,
        (ordinal, biome) => Some((
            ordinal.parse().unwrap(),
            roster[biome.parse::<usize>().unwrap()],
        )),
    }
}

fn parse_histories(roster: &[BiomeResourceId]) -> Vec<HistoryRow> {
    rows(HISTORIES, 13)
        .map(|r| HistoryRow {
            group: r[0].parse().unwrap(),
            sequence: r[1].parse().unwrap(),
            chunk: chunk(r[2], r[3]),
            history: r[4].to_owned(),
            settings: NoiseSettingsResourceId::from_name(r[5]).unwrap(),
            source: r[6].to_owned(),
            geometry: BiomeQuartGeometry::new(r[7].parse().unwrap(), r[8].parse().unwrap())
                .unwrap(),
            initial: optional_point(r[9], r[10], roster),
            last: optional_point(r[11], r[12], roster),
        })
        .collect()
}

fn parse() -> Fixture {
    let manifest: Value = serde_json::from_str(MANIFEST).unwrap();
    let roster: Vec<_> = manifest["roster"]
        .as_array()
        .unwrap()
        .iter()
        .map(|name| BiomeResourceId::from_name(name.as_str().unwrap()).unwrap())
        .collect();
    assert_eq!(roster.len(), 65);
    let groups: Vec<_> = manifest["groups"]
        .as_array()
        .unwrap()
        .iter()
        .map(|group| Group {
            id: group["id"].as_str().unwrap().to_owned(),
            axis: group["kind"] == "axis",
            center: ChunkPos::new(
                i32::try_from(group["center"][0].as_i64().unwrap()).unwrap(),
                i32::try_from(group["center"][1].as_i64().unwrap()).unwrap(),
            ),
        })
        .collect();
    let seeds: Vec<_> = manifest["seeds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|seed| Seed {
            world: seed["kind"] == "world",
            value: seed["value"].as_str().unwrap().parse().unwrap(),
            zoom_bits: seed["zoom_bits"].as_str().unwrap().parse().unwrap(),
        })
        .collect();
    let queries: Vec<_> = rows(QUERIES, 13)
        .map(|r| Query {
            group: r[0].parse().unwrap(),
            kind: r[1].as_bytes()[0],
            seed: r[2].parse().unwrap(),
            input: [
                r[3].parse().unwrap(),
                r[4].parse().unwrap(),
                r[5].parse().unwrap(),
            ],
            quart: StoredBiomeQuartPos::new(
                r[6].parse().unwrap(),
                r[7].parse().unwrap(),
                r[8].parse().unwrap(),
            ),
            chunk: chunk(r[9], r[10]),
            value: match r[11] {
                "V" => Some(roster[r[12].parse::<usize>().unwrap()]),
                "H" => {
                    assert_eq!(r[12], "-");
                    None
                }
                other => panic!("unsupported outcome {other}"),
            },
        })
        .collect();
    let arrays: Vec<_> = rows(ARRAYS, 6)
        .map(|r| ArrayRow {
            group: r[0].parse().unwrap(),
            chunk: chunk(r[1], r[2]),
            geometry: BiomeQuartGeometry::new(r[3].parse().unwrap(), r[4].parse().unwrap())
                .unwrap(),
            values: r[5]
                .split(',')
                .map(|v| roster[v.parse::<usize>().unwrap()])
                .collect(),
        })
        .collect();
    let histories = parse_histories(&roster);
    let evidence = manifest["excluded_exception_evidence"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["id"].as_str().unwrap().to_owned())
        .collect();
    let counts = &manifest["counts"];
    assert_eq!(queries.len() as u64, counts["queries"].as_u64().unwrap());
    assert_eq!(arrays.len() as u64, counts["arrays"].as_u64().unwrap());
    assert_eq!(
        histories.len() as u64,
        counts["histories"].as_u64().unwrap()
    );
    Fixture {
        groups,
        seeds,
        queries,
        arrays,
        histories,
        evidence,
        manifest,
    }
}

pub(super) fn fixture() -> &'static Fixture {
    static FIXTURE: OnceLock<Fixture> = OnceLock::new();
    FIXTURE.get_or_init(parse)
}

/// One genuine createBiomes neighborhood produced by the actual GEN-05.5 producers.
pub(super) struct Generated {
    pub group: usize,
    pub center: ChunkPos,
    pub identity: StoredBiomeIdentity,
    /// Actual arrays in the original generation sequence.
    pub arrays: Vec<ChunkBiomeSamples>,
}

impl Generated {
    pub fn refs(&self) -> Vec<&ChunkBiomeSamples> {
        self.arrays.iter().collect()
    }
}

pub(super) fn source_identity(row: &HistoryRow) -> BiomeSourceIdentity {
    match row.source.as_str() {
        "multinoise" => BiomeSourceIdentity::MultiNoise(
            BiomeParametersResourceId::from_name(
                if row.settings.resource_location() == "minecraft:nether" {
                    "minecraft:nether"
                } else {
                    "minecraft:overworld"
                },
            )
            .unwrap(),
        ),
        // The original single_biome_surface world preset is fixed plains.
        "fixed" => {
            BiomeSourceIdentity::Fixed(BiomeResourceId::from_name("minecraft:plains").unwrap())
        }
        "end" => BiomeSourceIdentity::End,
        other => panic!("unsupported original source {other}"),
    }
}

fn source_session(
    identity: BiomeSourceIdentity,
    trees: &HashMap<&'static str, Arc<BiomeTree>>,
) -> BiomeSourceSession {
    let budget = BiomeWorkBudget::new(MAX_BIOME_SELECTION_WORK).unwrap();
    match identity {
        BiomeSourceIdentity::MultiNoise(preset) => BiomeSourceSession::from_multinoise(
            BiomeSelectionSession::new(Arc::clone(&trees[preset.resource_location()]), budget),
        ),
        BiomeSourceIdentity::Fixed(biome) => BiomeSourceSession::fixed(biome, budget),
        BiomeSourceIdentity::End => BiomeSourceSession::the_end(budget).unwrap(),
    }
}

fn produce() -> Vec<Generated> {
    let fixture = fixture();
    let trees: HashMap<&'static str, Arc<BiomeTree>> = ["minecraft:overworld", "minecraft:nether"]
        .into_iter()
        .map(|name| {
            (
                name,
                Arc::new(
                    BiomeTree::from_builtin(
                        BiomeParametersResourceId::from_name(name).unwrap(),
                        BiomeBuildBudget::new(MAX_BIOME_BUILD_WORK).unwrap(),
                    )
                    .unwrap(),
                ),
            )
        })
        .collect();
    let mut routers: HashMap<(NoiseSettingsResourceId, i64), SeededRouter> = HashMap::new();
    let mut generated = Vec::new();
    for (index, group) in fixture.groups.iter().enumerate() {
        if group.axis {
            continue;
        }
        let rows: Vec<_> = fixture
            .histories
            .iter()
            .filter(|h| h.group == index)
            .collect();
        assert_eq!(rows.len(), 9);
        assert!(rows.iter().enumerate().all(|(i, h)| h.sequence == i));
        let first = rows[0];
        let seed: i64 = fixture.manifest["groups"][index]["world_seed"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let source = source_identity(first);
        let identity =
            StoredBiomeIdentity::new(WorldSeed::new(seed), first.settings, source, first.geometry);
        let router = routers.entry((first.settings, seed)).or_insert_with(|| {
            RouterTemplate::compile_builtin(first.settings)
                .unwrap()
                .bind_seed(WorldSeed::new(seed))
                .unwrap()
        });
        let shared = rows
            .iter()
            .all(|h| h.history == "forward" || h.history == "reverse");
        assert!(shared || rows.iter().all(|h| h.history == "cold"));
        let mut session = source_session(source, &trees);
        let mut arrays = Vec::new();
        for row in rows {
            assert_eq!(source_identity(row), source);
            assert_eq!(row.geometry, first.geometry);
            if !shared {
                // Separate cold sessions, exactly as the original cold groups.
                session = source_session(source, &trees);
            }
            let mut noise = router
                .fresh_paper_noise_chunk(
                    row.chunk,
                    NoiseChunkWorkBudget::new(MAX_NOISE_CHUNK_WORK).unwrap(),
                )
                .unwrap();
            arrays.push(
                session
                    .sample_chunk_biomes(&mut noise, row.geometry)
                    .unwrap(),
            );
        }
        generated.push(Generated {
            group: index,
            center: group.center,
            identity,
            arrays,
        });
    }
    assert_eq!(generated.len(), 20);
    generated
}

pub(super) fn generated() -> &'static [Generated] {
    static GENERATED: OnceLock<Vec<Generated>> = OnceLock::new();
    GENERATED.get_or_init(produce)
}

pub(super) fn generated_group(id: &str) -> &'static Generated {
    let fixture = fixture();
    generated()
        .iter()
        .find(|g| fixture.groups[g.group].id == id)
        .unwrap()
}

pub(super) fn budget(units: u32) -> StoredBiomeBuildBudget {
    StoredBiomeBuildBudget::new(units).unwrap()
}

pub(super) fn max_budget() -> StoredBiomeBuildBudget {
    budget(MAX_STORED_BIOME_BUILD_WORK)
}

/// Private malformed/axis control; reaches only the shared checked kernel.
#[derive(Debug, Clone)]
pub(super) struct Control {
    pub seed: WorldSeed,
    pub resource: NoiseSettingsResourceId,
    pub chunk: ChunkPos,
    pub geometry: BiomeQuartGeometry,
    pub source: BiomeSourceIdentity,
    pub values: Vec<BiomeResourceId>,
    pub initial: Option<HistoryPoint>,
    pub last: Option<HistoryPoint>,
}

impl StoredColumn for Control {
    fn column_seed(&self) -> WorldSeed {
        self.seed
    }
    fn column_resource(&self) -> NoiseSettingsResourceId {
        self.resource
    }
    fn column_chunk(&self) -> ChunkPos {
        self.chunk
    }
    fn column_geometry(&self) -> BiomeQuartGeometry {
        self.geometry
    }
    fn column_source(&self) -> BiomeSourceIdentity {
        self.source
    }
    fn column_values(&self) -> &[BiomeResourceId] {
        &self.values
    }
    fn column_initial(&self) -> Option<HistoryPoint> {
        self.initial
    }
    fn column_final(&self) -> Option<HistoryPoint> {
        self.last
    }
}

pub(super) fn settings(name: &str) -> NoiseSettingsResourceId {
    NoiseSettingsResourceId::from_name(name).unwrap()
}

pub(super) fn biome(name: &str) -> BiomeResourceId {
    BiomeResourceId::from_name(name).unwrap()
}

/// Control identity used by synthetic policy controls (End: no history/value shape).
pub(super) fn control_identity(geometry: BiomeQuartGeometry) -> StoredBiomeIdentity {
    StoredBiomeIdentity::new(
        WorldSeed::new(0),
        settings("minecraft:overworld"),
        BiomeSourceIdentity::End,
        geometry,
    )
}

pub(super) fn control(identity: StoredBiomeIdentity, chunk: ChunkPos) -> Control {
    let count = identity.geometry().sample_count();
    Control {
        seed: identity.seed(),
        resource: identity.resource(),
        chunk,
        geometry: identity.geometry(),
        source: identity.source(),
        values: (0..count)
            .map(|i| {
                ferrumc_registry::biome::biome_resources()
                    .nth(i % 65)
                    .unwrap()
            })
            .collect(),
        initial: None,
        last: None,
    }
}

/// All nine window controls in slot order.
pub(super) fn window(identity: StoredBiomeIdentity, center: ChunkPos) -> Vec<Control> {
    (0..MAX_STORED_BIOME_CHUNKS)
        .map(|slot| control(identity, slot_key(center, slot).unwrap()))
        .collect()
}

pub(super) fn admit_controls<'a>(
    identity: StoredBiomeIdentity,
    center: ChunkPos,
    controls: &[&'a Control],
    units: u32,
) -> Result<Core<'a, Control>, StoredBiomeError> {
    Core::admit(identity, center, controls, budget(units), 0, 0)
}

pub(super) fn observed_zoom() -> Option<BiomeZoom> {
    OBSERVED_ZOOM.with(std::cell::Cell::take)
}
