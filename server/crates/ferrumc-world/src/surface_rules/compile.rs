//! Bounded compilation of the original overworld `surface_rule` export.
//!
//! One checked parse of the pinned Registry settings definition, one preorder
//! walk (the original `RuleSource.apply` / oracle node order) and separate
//! reference passes. No hand transcription; the Paper data substitution of the
//! bedrock gradients is applied mechanically and stays visible in the table.

use std::collections::BTreeMap;
use std::mem::{size_of, size_of_val};

use ferrumc_registry::biome::BiomeResourceId;
use ferrumc_registry::block_state::compute_state_id;
use ferrumc_registry::worldgen::{NoiseResourceId, NoiseSettingsResourceId, MAX_DEFINITION_BYTES};

use super::error::limit;
use super::{
    SurfaceRuleBuildBudget, SurfaceRuleError, SurfaceRuleErrorKind as Kind, SurfaceRuleLimit,
    MAX_SURFACE_RULE_BIOMES_PER_CONDITION, MAX_SURFACE_RULE_BIOME_REFERENCES,
    MAX_SURFACE_RULE_DEPTH, MAX_SURFACE_RULE_NODES, MAX_SURFACE_RULE_NOISES,
    MAX_SURFACE_RULE_RANDOM_NAMES, MAX_SURFACE_RULE_SEQUENCE_CHILDREN, MAX_SURFACE_RULE_STATES,
    MAX_SURFACE_RULE_TEMPLATE_BYTES, SURFACE_RULE_LEVEL_HEIGHT, SURFACE_RULE_LEVEL_MIN_Y,
};
use crate::density::{
    decode_double, decode_int, parse_definition, DensityCompileError, DensityCompileErrorKind,
    DensityLimit, JsonValue, JsonValueKind,
};
use crate::BlockStateId;

/// The three admitted overworld settings; their trees are canonically equal (P3).
pub(super) const ADMITTED_SETTINGS: [&str; 3] = [
    "minecraft:overworld",
    "minecraft:large_biomes",
    "minecraft:amplified",
];
/// `SurfaceSystem` noises bound before the tree's own threshold noises.
pub(super) const SYSTEM_NOISES: [&str; 3] = [
    "minecraft:surface",
    "minecraft:surface_secondary",
    "minecraft:clay_bands_offset",
];
pub(super) const SURFACE_NOISE: usize = 0;
pub(super) const SECONDARY_NOISE: usize = 1;
pub(super) const CLAY_NOISE: usize = 2;
/// Pinned compiled census of the admitted trees: nodes and resolved references
/// (100 block rules + 53 biome names + 33 noises + 2 random names + 14 anchors).
pub(super) const ADMITTED_TREE_NODES: usize = 428;
pub(super) const ADMITTED_TREE_REFERENCES: usize = 202;
/// Original `VerticalAnchor` codec range `DimensionType.MIN_Y..=MAX_Y`.
const ANCHOR_RANGE: (i32, i32) = (-2032, 2031);
/// Original `Codec.intRange(-20, 20)` of `surface_depth_multiplier`.
const MULTIPLIER_RANGE: (i32, i32) = (-20, 20);
const NO_PARENT: u16 = u16::MAX;

/// Compiled node kind with payload indices into the template tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NodeKind {
    Sequence { first: u16, len: u16 },
    Condition { test: u16, then: u16 },
    Block(u16),
    Bandlands,
    Biome { first: u16, len: u16 },
    NoiseThreshold(u16),
    VerticalGradient(u16),
    PaperFlatBedrock(u16),
    YAbove(u16),
    Water(u16),
    StoneDepth(u16),
    Steep,
    Hole,
    AbovePreliminarySurface,
    Temperature,
    Not(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Node {
    pub kind: NodeKind,
    pub parent: u16,
    pub depth: u8,
}

/// Original `VerticalAnchor` before resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Anchor {
    Absolute(i32),
    AboveBottom(i32),
    BelowTop(i32),
}

impl Anchor {
    /// Original `resolveY` with Java `int` wrapping.
    const fn resolve(self, generation: Generation) -> i32 {
        match self {
            Self::Absolute(y) => y,
            Self::AboveBottom(offset) => generation.min_y.wrapping_add(offset),
            Self::BelowTop(offset) => generation
                .depth
                .wrapping_sub(1)
                .wrapping_add(generation.min_y)
                .wrapping_sub(offset),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Threshold {
    pub noise: u8,
    pub min: f64,
    pub max: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Gradient {
    pub random: u8,
    pub true_at: Anchor,
    pub false_at: Anchor,
    pub true_y: i32,
    pub false_y: i32,
    /// Paper substitution role: `Some(is_roof)` for `bedrock_floor`/`bedrock_roof`.
    pub paper_roof: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct YCheck {
    pub anchor: Anchor,
    pub y: i32,
    pub multiplier: i32,
    pub add_stone: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct WaterCheck {
    pub offset: i32,
    pub multiplier: i32,
    pub add_stone: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct StoneCheck {
    pub offset: i32,
    pub add_surface: bool,
    pub range: i32,
    pub ceiling: bool,
}

/// Original `WorldGenerationContext` for the assumed level height, and sea level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Generation {
    pub min_y: i32,
    pub depth: i32,
    pub sea_level: i32,
    pub settings_min_y: i32,
    pub settings_height: i32,
}

/// Construction census: definition bytes + JSON nodes + compiled nodes + references.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Census {
    pub definition_bytes: usize,
    pub json_nodes: usize,
    pub nodes: usize,
    pub references: usize,
    pub depth: usize,
}

impl Census {
    pub(super) const fn work(self) -> usize {
        self.definition_bytes + self.json_nodes + self.nodes + self.references
    }
}

/// Native capacities applied by one compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Caps {
    pub nodes: usize,
    pub depth: usize,
    pub sequence_children: usize,
    pub biome_references: usize,
    pub biomes_per_condition: usize,
    pub noises: usize,
    pub random_names: usize,
    pub states: usize,
    pub template_bytes: usize,
}

impl Caps {
    pub(super) const PRODUCTION: Self = Self {
        nodes: MAX_SURFACE_RULE_NODES,
        depth: MAX_SURFACE_RULE_DEPTH,
        sequence_children: MAX_SURFACE_RULE_SEQUENCE_CHILDREN,
        biome_references: MAX_SURFACE_RULE_BIOME_REFERENCES,
        biomes_per_condition: MAX_SURFACE_RULE_BIOMES_PER_CONDITION,
        noises: MAX_SURFACE_RULE_NOISES,
        random_names: MAX_SURFACE_RULE_RANDOM_NAMES,
        states: MAX_SURFACE_RULE_STATES,
        template_bytes: MAX_SURFACE_RULE_TEMPLATE_BYTES,
    };
}

/// Immutable seed-free compiled overworld surface-rule tree.
///
/// Nodes are kept in the original preorder (a single-child `sequence` keeps
/// its node and evaluates as its child). Block, biome and noise identities are
/// distinct checked types; no quart or wire ID appears. Bootstrap object
/// sharing is result-neutral and not retained.
#[derive(Debug)]
pub struct SurfaceRuleTemplate {
    resource: NoiseSettingsResourceId,
    pub(super) nodes: Box<[Node]>,
    pub(super) children: Box<[u16]>,
    pub(super) states: Box<[BlockStateId]>,
    pub(super) biomes: Box<[BiomeResourceId]>,
    pub(super) thresholds: Box<[Threshold]>,
    pub(super) gradients: Box<[Gradient]>,
    pub(super) y_checks: Box<[YCheck]>,
    pub(super) water_checks: Box<[WaterCheck]>,
    pub(super) stone_checks: Box<[StoneCheck]>,
    pub(super) noises: Box<[NoiseResourceId]>,
    pub(super) random_names: Box<[Box<str>]>,
    pub(super) generation: Generation,
    pub(super) census: Census,
    pub(super) evaluate_work: u32,
}

impl SurfaceRuleTemplate {
    /// Compiles the `surface_rule` of an admitted builtin overworld setting.
    ///
    /// Refusal order: the census allowance (`WorkBudgetExceeded`, from the
    /// Registry definition statistics and the pinned tree census), settings
    /// admission (P3), source limits before parsing, JSON admission, settings
    /// fields, codec shape, references (blocks, biomes, noises, random names),
    /// census limits, pinned census drift and the retained-byte cap.
    pub fn compile_builtin(
        resource: NoiseSettingsResourceId,
        budget: SurfaceRuleBuildBudget,
    ) -> Result<Self, SurfaceRuleError> {
        compile_with(resource, budget, Caps::PRODUCTION)
    }
    /// Settings resource this tree was compiled from.
    pub const fn resource(&self) -> NoiseSettingsResourceId {
        self.resource
    }
    /// Compiled nodes (428 for the admitted overworld trees).
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
    /// Spent construction census: definition bytes + JSON nodes + nodes + references.
    pub const fn construction_work(&self) -> usize {
        self.census.work()
    }
    /// Worst-case units of one `evaluate` call without the minimum surface
    /// (467 for the admitted overworld trees).
    pub const fn evaluate_work(&self) -> u32 {
        self.evaluate_work
    }
    /// Original sea level of the settings.
    pub const fn sea_level(&self) -> i32 {
        self.generation.sea_level
    }
    /// Generation-context minimum block Y (`max(level, settings)`).
    pub const fn min_generation_y(&self) -> i32 {
        self.generation.min_y
    }
    /// Generation-context depth (`min(level, settings)`).
    pub const fn generation_depth(&self) -> i32 {
        self.generation.depth
    }
    /// Measured retained native bytes: the value and every owned table.
    pub fn retained_bytes(&self) -> usize {
        size_of::<Self>()
            + size_of_val(&*self.nodes)
            + size_of_val(&*self.children)
            + size_of_val(&*self.states)
            + size_of_val(&*self.biomes)
            + size_of_val(&*self.thresholds)
            + size_of_val(&*self.gradients)
            + size_of_val(&*self.y_checks)
            + size_of_val(&*self.water_checks)
            + size_of_val(&*self.stone_checks)
            + size_of_val(&*self.noises)
            + size_of_val(&*self.random_names)
            + self.random_names.iter().map(|n| n.len()).sum::<usize>()
    }
}

pub(super) fn compile_with(
    resource: NoiseSettingsResourceId,
    budget: SurfaceRuleBuildBudget,
    caps: Caps,
) -> Result<SurfaceRuleTemplate, SurfaceRuleError> {
    let definition = resource.definition();
    let stats = definition.stats();
    let requirement =
        stats.bytes() + stats.nodes() + ADMITTED_TREE_NODES + ADMITTED_TREE_REFERENCES;
    if (budget.units() as usize) < requirement {
        return Err(SurfaceRuleError::new(Kind::WorkBudgetExceeded)
            .with_counts(requirement, budget.units() as usize));
    }
    let name = resource.resource_location();
    if !ADMITTED_SETTINGS.contains(&name) {
        return Err(SurfaceRuleError::new(Kind::UnsupportedSettings));
    }
    let json = definition.json();
    limit(
        SurfaceRuleLimit::DefinitionBytes,
        json.len(),
        MAX_DEFINITION_BYTES,
    )?;
    let (root, parsed) = parse_definition(json, name).map_err(json_error)?;
    if parsed.nodes != stats.nodes() || json.len() != stats.bytes() {
        return Err(SurfaceRuleError::new(Kind::MalformedDefinition));
    }
    let (generation, rule) = settings(&root, name)?;
    let mut template = Builder::new(name, generation, caps).compile(rule, resource)?;
    let census = Census {
        definition_bytes: json.len(),
        json_nodes: parsed.nodes,
        ..template.census
    };
    // Drift against the pinned, canonically equal admitted tree census.
    if census.nodes != ADMITTED_TREE_NODES || census.references != ADMITTED_TREE_REFERENCES {
        return Err(SurfaceRuleError::new(Kind::MalformedDefinition)
            .with_counts(census.nodes, ADMITTED_TREE_NODES));
    }
    template.census = census;
    limit(
        SurfaceRuleLimit::TemplateBytes,
        template.retained_bytes(),
        caps.template_bytes,
    )?;
    Ok(template)
}

/// Compiles a bare synthetic `surface_rule` JSON (test boundaries only).
#[cfg(test)]
pub(super) fn compile_synthetic(
    rule_json: &str,
    caps: Caps,
) -> Result<SurfaceRuleTemplate, SurfaceRuleError> {
    let (root, parsed) = parse_definition(rule_json, "synthetic").map_err(json_error)?;
    let generation = Generation {
        min_y: SURFACE_RULE_LEVEL_MIN_Y,
        depth: SURFACE_RULE_LEVEL_HEIGHT.cast_signed(),
        sea_level: 63,
        settings_min_y: SURFACE_RULE_LEVEL_MIN_Y,
        settings_height: SURFACE_RULE_LEVEL_HEIGHT.cast_signed(),
    };
    let resource =
        NoiseSettingsResourceId::from_name("minecraft:overworld").ok_or_else(malformed)?;
    let mut template = Builder::new("synthetic", generation, caps).compile(&root, resource)?;
    template.census.definition_bytes = rule_json.len();
    template.census.json_nodes = parsed.nodes;
    limit(
        SurfaceRuleLimit::TemplateBytes,
        template.retained_bytes(),
        caps.template_bytes,
    )?;
    Ok(template)
}

#[allow(clippy::needless_pass_by_value)] // used directly as a `map_err` adapter
fn json_error(error: DensityCompileError) -> SurfaceRuleError {
    let failure = SurfaceRuleError::new(Kind::MalformedDefinition);
    let DensityCompileErrorKind::Limit {
        limit,
        count,
        maximum,
    } = *error.kind()
    else {
        return failure;
    };
    let limit = match limit {
        DensityLimit::DefinitionBytes => SurfaceRuleLimit::DefinitionBytes,
        DensityLimit::JsonNodes => SurfaceRuleLimit::JsonNodes,
        DensityLimit::JsonDepth => SurfaceRuleLimit::JsonDepth,
        DensityLimit::ContainerEntries => SurfaceRuleLimit::ContainerEntries,
        DensityLimit::StringBytes => SurfaceRuleLimit::StringBytes,
        DensityLimit::IntegerTokenBytes => SurfaceRuleLimit::IntegerTokenBytes,
        _ => return failure,
    };
    SurfaceRuleError::new(Kind::NativeLimit)
        .with_limit(limit)
        .with_counts(count, maximum)
}

fn malformed() -> SurfaceRuleError {
    SurfaceRuleError::new(Kind::MalformedDefinition)
}

type Object<'v, 'a> = &'v BTreeMap<String, JsonValue<'a>>;

fn object<'v, 'a>(value: &'v JsonValue<'a>) -> Result<Object<'v, 'a>, SurfaceRuleError> {
    match &value.kind {
        JsonValueKind::Object(map) => Ok(map),
        _ => Err(malformed()),
    }
}

/// Exactly the listed fields; extra fields are refused (`FerrumC` admission policy).
fn exact(map: Object<'_, '_>, fields: &[&str]) -> Result<(), SurfaceRuleError> {
    if map.len() != fields.len() || fields.iter().any(|f| !map.contains_key(*f)) {
        return Err(malformed());
    }
    Ok(())
}

fn field<'v, 'a>(map: Object<'v, 'a>, name: &str) -> Result<&'v JsonValue<'a>, SurfaceRuleError> {
    map.get(name).ok_or_else(malformed)
}

fn string<'v>(value: &'v JsonValue<'_>) -> Result<&'v str, SurfaceRuleError> {
    match &value.kind {
        JsonValueKind::String(text) => Ok(text),
        _ => Err(malformed()),
    }
}

fn boolean(value: &JsonValue<'_>) -> Result<bool, SurfaceRuleError> {
    match value.kind {
        JsonValueKind::Bool(flag) => Ok(flag),
        _ => Err(malformed()),
    }
}

fn int(value: &JsonValue<'_>, resource: &'static str) -> Result<i32, SurfaceRuleError> {
    decode_int(value, resource, "surface_rule", i32::MIN, i32::MAX).map_err(json_error)
}

fn double(value: &JsonValue<'_>, resource: &'static str) -> Result<f64, SurfaceRuleError> {
    decode_double(value, resource, "surface_rule", f64::MIN, f64::MAX).map_err(json_error)
}

/// Settings fields this ticket consumes; the rest stays with the density compiler.
fn settings<'v, 'a>(
    root: &'v JsonValue<'a>,
    resource: &'static str,
) -> Result<(Generation, &'v JsonValue<'a>), SurfaceRuleError> {
    let map = object(root)?;
    if boolean(field(map, "legacy_random_source")?)? {
        return Err(SurfaceRuleError::new(Kind::UnsupportedSettings));
    }
    let sea_level = int(field(map, "sea_level")?, resource)?;
    let noise = object(field(map, "noise")?)?;
    let settings_min_y = int(field(noise, "min_y")?, resource)?;
    let settings_height = int(field(noise, "height")?, resource)?;
    // Original WorldGenerationContext over the assumed level height (-64, 384).
    let generation = Generation {
        min_y: SURFACE_RULE_LEVEL_MIN_Y.max(settings_min_y),
        depth: SURFACE_RULE_LEVEL_HEIGHT.cast_signed().min(settings_height),
        sea_level,
        settings_min_y,
        settings_height,
    };
    Ok((generation, field(map, "surface_rule")?))
}

fn codec_type<'v>(map: Object<'v, '_>) -> Result<&'v str, SurfaceRuleError> {
    let name = string(field(map, "type")?)?;
    Ok(name.strip_prefix("minecraft:").unwrap_or(name))
}

/// Original `ResourceLocation` syntax; bare paths take the `minecraft` namespace.
fn resource_location(name: &str) -> Option<String> {
    let (namespace, path) = name.split_once(':').unwrap_or(("minecraft", name));
    let valid = |text: &str, slash: bool| {
        text.bytes().all(|b| {
            b.is_ascii_lowercase()
                || b.is_ascii_digit()
                || matches!(b, b'_' | b'-' | b'.')
                || (slash && b == b'/')
        })
    };
    (!path.is_empty() && valid(namespace, false) && valid(path, true))
        .then(|| format!("{namespace}:{path}"))
}

enum Pending<'v, 'a> {
    Block(&'v JsonValue<'a>),
    Biomes(Vec<&'v str>),
    Noise(&'v str),
    Random(&'v str),
}

struct Builder<'v, 'a> {
    resource: &'static str,
    generation: Generation,
    caps: Caps,
    nodes: Vec<Node>,
    children: Vec<u16>,
    pending: Vec<(usize, Pending<'v, 'a>)>,
    thresholds: Vec<Threshold>,
    gradients: Vec<Gradient>,
    y_checks: Vec<YCheck>,
    water_checks: Vec<WaterCheck>,
    stone_checks: Vec<StoneCheck>,
    anchors: usize,
    depth: usize,
    max_children: usize,
}

fn index(value: usize) -> Result<u16, SurfaceRuleError> {
    // The JSON admission bounds every table far below u16; this is defensive.
    u16::try_from(value).map_err(|_| malformed())
}

impl<'v, 'a> Builder<'v, 'a> {
    fn new(resource: &'static str, generation: Generation, caps: Caps) -> Self {
        Self {
            resource,
            generation,
            caps,
            nodes: Vec::new(),
            children: Vec::new(),
            pending: Vec::new(),
            thresholds: Vec::new(),
            gradients: Vec::new(),
            y_checks: Vec::new(),
            water_checks: Vec::new(),
            stone_checks: Vec::new(),
            anchors: 0,
            depth: 0,
            max_children: 0,
        }
    }

    fn compile(
        mut self,
        rule: &'v JsonValue<'a>,
        resource: NoiseSettingsResourceId,
    ) -> Result<SurfaceRuleTemplate, SurfaceRuleError> {
        self.rule(rule, NO_PARENT, 0)?;
        // Blocks, biome names, noises and random names per occurrence, plus anchors.
        let references = self
            .pending
            .iter()
            .map(|(_, p)| match p {
                Pending::Biomes(names) => names.len(),
                _ => 1,
            })
            .sum::<usize>()
            + self.anchors;
        let resolved = self.resolve()?;
        self.census_limits(&resolved)?;
        let evaluate_work = self.evaluate_work()?;
        Ok(SurfaceRuleTemplate {
            resource,
            census: Census {
                definition_bytes: 0,
                json_nodes: 0,
                nodes: self.nodes.len(),
                references,
                depth: self.depth,
            },
            nodes: resolved.nodes.into_boxed_slice(),
            children: self.children.into_boxed_slice(),
            states: resolved.states.into_boxed_slice(),
            biomes: resolved.biomes.into_boxed_slice(),
            thresholds: self.thresholds.into_boxed_slice(),
            gradients: resolved.gradients.into_boxed_slice(),
            y_checks: self.y_checks.into_boxed_slice(),
            water_checks: self.water_checks.into_boxed_slice(),
            stone_checks: self.stone_checks.into_boxed_slice(),
            noises: resolved.noises.into_boxed_slice(),
            random_names: resolved.random_names.into_boxed_slice(),
            generation: self.generation,
            evaluate_work,
        })
    }

    fn push(
        &mut self,
        kind: NodeKind,
        parent: u16,
        depth: usize,
    ) -> Result<usize, SurfaceRuleError> {
        let id = self.nodes.len();
        // Bounded by the JSON admission; the census cap is applied after references.
        self.depth = self.depth.max(depth);
        self.nodes.push(Node {
            kind,
            parent,
            depth: u8::try_from(depth).map_err(|_| malformed())?,
        });
        Ok(id)
    }

    fn rule(
        &mut self,
        value: &'v JsonValue<'a>,
        parent: u16,
        depth: usize,
    ) -> Result<u16, SurfaceRuleError> {
        let map = object(value)?;
        let kind = codec_type(map)?;
        let id = match kind {
            "sequence" => {
                exact(map, &["type", "sequence"])?;
                let JsonValueKind::Array(items) = &field(map, "sequence")?.kind else {
                    return Err(malformed());
                };
                let id = self.push(NodeKind::Sequence { first: 0, len: 0 }, parent, depth)?;
                let mut list = Vec::with_capacity(items.len());
                for item in items {
                    list.push(self.rule(item, index(id)?, depth + 1)?);
                }
                self.max_children = self.max_children.max(list.len());
                let first = index(self.children.len())?;
                self.children.extend_from_slice(&list);
                self.nodes[id].kind = NodeKind::Sequence {
                    first,
                    len: index(list.len())?,
                };
                id
            }
            "condition" => {
                exact(map, &["type", "if_true", "then_run"])?;
                let id = self.push(NodeKind::Condition { test: 0, then: 0 }, parent, depth)?;
                let test = self.condition(field(map, "if_true")?, index(id)?, depth + 1)?;
                let then = self.rule(field(map, "then_run")?, index(id)?, depth + 1)?;
                self.nodes[id].kind = NodeKind::Condition { test, then };
                id
            }
            "block" => {
                exact(map, &["type", "result_state"])?;
                let id = self.push(NodeKind::Block(0), parent, depth)?;
                self.pending
                    .push((id, Pending::Block(field(map, "result_state")?)));
                id
            }
            "bandlands" => {
                exact(map, &["type"])?;
                self.push(NodeKind::Bandlands, parent, depth)?
            }
            _ => {
                return Err(
                    SurfaceRuleError::new(Kind::UnknownCodecType).with_node(self.nodes.len())
                )
            }
        };
        index(id)
    }

    fn condition(
        &mut self,
        value: &'v JsonValue<'a>,
        parent: u16,
        depth: usize,
    ) -> Result<u16, SurfaceRuleError> {
        let map = object(value)?;
        let id = match codec_type(map)? {
            "not" => {
                exact(map, &["type", "invert"])?;
                let id = self.push(NodeKind::Not(0), parent, depth)?;
                let target = self.condition(field(map, "invert")?, index(id)?, depth + 1)?;
                self.nodes[id].kind = NodeKind::Not(target);
                id
            }
            "biome" => {
                exact(map, &["type", "biome_is"])?;
                let JsonValueKind::Array(items) = &field(map, "biome_is")?.kind else {
                    return Err(malformed());
                };
                let names = items.iter().map(string).collect::<Result<Vec<_>, _>>()?;
                let id = self.push(NodeKind::Biome { first: 0, len: 0 }, parent, depth)?;
                self.pending.push((id, Pending::Biomes(names)));
                id
            }
            "noise_threshold" => self.noise_threshold(map, parent, depth)?,
            "vertical_gradient" => self.gradient(map, parent, depth)?,
            "y_above" => self.y_above(map, parent, depth)?,
            "water" => self.water(map, parent, depth)?,
            "stone_depth" => self.stone_depth(map, parent, depth)?,
            unit @ ("steep" | "hole" | "above_preliminary_surface" | "temperature") => {
                exact(map, &["type"])?;
                let kind = match unit {
                    "steep" => NodeKind::Steep,
                    "hole" => NodeKind::Hole,
                    "temperature" => NodeKind::Temperature,
                    _ => NodeKind::AbovePreliminarySurface,
                };
                self.push(kind, parent, depth)?
            }
            _ => {
                return Err(
                    SurfaceRuleError::new(Kind::UnknownCodecType).with_node(self.nodes.len())
                )
            }
        };
        index(id)
    }

    fn noise_threshold(
        &mut self,
        map: Object<'v, 'a>,
        parent: u16,
        depth: usize,
    ) -> Result<usize, SurfaceRuleError> {
        exact(map, &["type", "noise", "min_threshold", "max_threshold"])?;
        let min = double(field(map, "min_threshold")?, self.resource)?;
        let max = double(field(map, "max_threshold")?, self.resource)?;
        let slot = index(self.thresholds.len())?;
        self.thresholds.push(Threshold { noise: 0, min, max });
        let id = self.push(NodeKind::NoiseThreshold(slot), parent, depth)?;
        self.pending
            .push((id, Pending::Noise(string(field(map, "noise")?)?)));
        Ok(id)
    }

    fn anchor(&mut self, value: &JsonValue<'_>) -> Result<Anchor, SurfaceRuleError> {
        let node = self.nodes.len();
        let invalid = || SurfaceRuleError::new(Kind::InvalidAnchor).with_node(node);
        let JsonValueKind::Object(map) = &value.kind else {
            return Err(invalid());
        };
        let mut entries = map.iter();
        let (Some((key, value)), None) = (entries.next(), entries.next()) else {
            return Err(invalid());
        };
        let raw = decode_int(
            value,
            self.resource,
            "anchor",
            ANCHOR_RANGE.0,
            ANCHOR_RANGE.1,
        )
        .map_err(|_| invalid())?;
        self.anchors += 1;
        match key.as_str() {
            "absolute" => Ok(Anchor::Absolute(raw)),
            "above_bottom" => Ok(Anchor::AboveBottom(raw)),
            "below_top" => Ok(Anchor::BelowTop(raw)),
            _ => Err(invalid()),
        }
    }

    fn gradient(
        &mut self,
        map: Object<'v, 'a>,
        parent: u16,
        depth: usize,
    ) -> Result<usize, SurfaceRuleError> {
        exact(
            map,
            &[
                "type",
                "random_name",
                "true_at_and_below",
                "false_at_and_above",
            ],
        )?;
        let true_at = self.anchor(field(map, "true_at_and_below")?)?;
        let false_at = self.anchor(field(map, "false_at_and_above")?)?;
        let slot = index(self.gradients.len())?;
        self.gradients.push(Gradient {
            random: 0,
            true_at,
            false_at,
            true_y: true_at.resolve(self.generation),
            false_y: false_at.resolve(self.generation),
            paper_roof: None,
        });
        let id = self.push(NodeKind::VerticalGradient(slot), parent, depth)?;
        self.pending
            .push((id, Pending::Random(string(field(map, "random_name")?)?)));
        Ok(id)
    }

    fn multiplier(&self, value: &JsonValue<'_>) -> Result<i32, SurfaceRuleError> {
        let raw = int(value, self.resource)?;
        if raw < MULTIPLIER_RANGE.0 || raw > MULTIPLIER_RANGE.1 {
            return Err(malformed());
        }
        Ok(raw)
    }

    fn y_above(
        &mut self,
        map: Object<'v, 'a>,
        parent: u16,
        depth: usize,
    ) -> Result<usize, SurfaceRuleError> {
        exact(
            map,
            &[
                "type",
                "anchor",
                "surface_depth_multiplier",
                "add_stone_depth",
            ],
        )?;
        let anchor = self.anchor(field(map, "anchor")?)?;
        let check = YCheck {
            anchor,
            y: anchor.resolve(self.generation),
            multiplier: self.multiplier(field(map, "surface_depth_multiplier")?)?,
            add_stone: boolean(field(map, "add_stone_depth")?)?,
        };
        let slot = index(self.y_checks.len())?;
        self.y_checks.push(check);
        self.push(NodeKind::YAbove(slot), parent, depth)
    }

    fn water(
        &mut self,
        map: Object<'v, 'a>,
        parent: u16,
        depth: usize,
    ) -> Result<usize, SurfaceRuleError> {
        exact(
            map,
            &[
                "type",
                "offset",
                "surface_depth_multiplier",
                "add_stone_depth",
            ],
        )?;
        let check = WaterCheck {
            offset: int(field(map, "offset")?, self.resource)?,
            multiplier: self.multiplier(field(map, "surface_depth_multiplier")?)?,
            add_stone: boolean(field(map, "add_stone_depth")?)?,
        };
        let slot = index(self.water_checks.len())?;
        self.water_checks.push(check);
        self.push(NodeKind::Water(slot), parent, depth)
    }

    fn stone_depth(
        &mut self,
        map: Object<'v, 'a>,
        parent: u16,
        depth: usize,
    ) -> Result<usize, SurfaceRuleError> {
        exact(
            map,
            &[
                "type",
                "offset",
                "add_surface_depth",
                "secondary_depth_range",
                "surface_type",
            ],
        )?;
        let ceiling = match string(field(map, "surface_type")?)? {
            "ceiling" => true,
            "floor" => false,
            _ => return Err(malformed()),
        };
        let check = StoneCheck {
            offset: int(field(map, "offset")?, self.resource)?,
            add_surface: boolean(field(map, "add_surface_depth")?)?,
            range: int(field(map, "secondary_depth_range")?, self.resource)?,
            ceiling,
        };
        let slot = index(self.stone_checks.len())?;
        self.stone_checks.push(check);
        self.push(NodeKind::StoneDepth(slot), parent, depth)
    }
}

struct Resolved {
    nodes: Vec<Node>,
    states: Vec<BlockStateId>,
    biomes: Vec<BiomeResourceId>,
    gradients: Vec<Gradient>,
    noises: Vec<NoiseResourceId>,
    random_names: Vec<Box<str>>,
}

impl Builder<'_, '_> {
    /// Reference passes in the declared order: blocks, biomes, noises, random names.
    fn resolve(&mut self) -> Result<Resolved, SurfaceRuleError> {
        let mut out = Resolved {
            nodes: self.nodes.clone(),
            states: Vec::new(),
            biomes: Vec::new(),
            gradients: self.gradients.clone(),
            noises: Vec::new(),
            random_names: Vec::new(),
        };
        for (node, pending) in &self.pending {
            if let Pending::Block(value) = pending {
                let state = block_state(value).ok_or_else(|| {
                    SurfaceRuleError::new(Kind::InvalidBlockState).with_node(*node)
                })?;
                let slot = intern(&mut out.states, state);
                out.nodes[*node].kind = NodeKind::Block(index(slot)?);
            }
        }
        for (node, pending) in &self.pending {
            if let Pending::Biomes(names) = pending {
                let first = index(out.biomes.len())?;
                for name in names {
                    out.biomes
                        .push(BiomeResourceId::from_name(name).ok_or_else(|| {
                            SurfaceRuleError::new(Kind::UnknownBiome).with_node(*node)
                        })?);
                }
                out.nodes[*node].kind = NodeKind::Biome {
                    first,
                    len: index(names.len())?,
                };
            }
        }
        out.noises = SYSTEM_NOISES
            .iter()
            .map(|name| NoiseResourceId::from_name(name).ok_or_else(malformed))
            .collect::<Result<_, _>>()?;
        for (node, pending) in &self.pending {
            if let Pending::Noise(name) = pending {
                let noise = NoiseResourceId::from_name(name)
                    .ok_or_else(|| SurfaceRuleError::new(Kind::UnknownNoise).with_node(*node))?;
                let slot = intern(&mut out.noises, noise);
                if let NodeKind::NoiseThreshold(threshold) = out.nodes[*node].kind {
                    self.thresholds[usize::from(threshold)].noise =
                        u8::try_from(slot).map_err(|_| malformed())?;
                }
            }
        }
        self.resolve_random(&mut out)?;
        Ok(out)
    }

    fn resolve_random(&self, out: &mut Resolved) -> Result<(), SurfaceRuleError> {
        for (node, pending) in &self.pending {
            if let Pending::Random(name) = pending {
                let name = resource_location(name).ok_or_else(|| {
                    SurfaceRuleError::new(Kind::MalformedDefinition).with_node(*node)
                })?;
                let slot = intern(&mut out.random_names, name.clone().into_boxed_str());
                let NodeKind::VerticalGradient(gradient) = out.nodes[*node].kind else {
                    return Err(malformed());
                };
                let entry = &mut out.gradients[usize::from(gradient)];
                entry.random = u8::try_from(slot).map_err(|_| malformed())?;
                // Paper data substitution (pinned resource patches), mechanically.
                entry.paper_roof = match name.as_str() {
                    "minecraft:bedrock_floor" => Some(false),
                    "minecraft:bedrock_roof" => Some(true),
                    _ => None,
                };
                if entry.paper_roof.is_some() {
                    out.nodes[*node].kind = NodeKind::PaperFlatBedrock(gradient);
                }
            }
        }
        Ok(())
    }

    fn census_limits(&self, resolved: &Resolved) -> Result<(), SurfaceRuleError> {
        let caps = self.caps;
        let biome_references = resolved.biomes.len();
        let widest = self
            .pending
            .iter()
            .map(|(_, p)| match p {
                Pending::Biomes(names) => names.len(),
                _ => 0,
            })
            .max()
            .unwrap_or(0);
        limit(SurfaceRuleLimit::Nodes, self.nodes.len(), caps.nodes)?;
        limit(SurfaceRuleLimit::Depth, self.depth, caps.depth)?;
        limit(
            SurfaceRuleLimit::SequenceChildren,
            self.max_children,
            caps.sequence_children,
        )?;
        limit(
            SurfaceRuleLimit::BiomeReferences,
            biome_references,
            caps.biome_references,
        )?;
        limit(
            SurfaceRuleLimit::BiomesPerCondition,
            widest,
            caps.biomes_per_condition,
        )?;
        limit(SurfaceRuleLimit::Noises, resolved.noises.len(), caps.noises)?;
        limit(
            SurfaceRuleLimit::RandomNames,
            resolved.random_names.len(),
            caps.random_names,
        )?;
        limit(SurfaceRuleLimit::States, resolved.states.len(), caps.states)
    }

    /// Worst case of one call: every rule and condition node, every threshold
    /// sample and gradient draw, plus one secondary, band, biome and temperature.
    fn evaluate_work(&self) -> Result<u32, SurfaceRuleError> {
        let count = |pred: fn(NodeKind) -> bool| self.nodes.iter().filter(|n| pred(n.kind)).count();
        let any = |pred: fn(NodeKind) -> bool| usize::from(count(pred) > 0);
        let secondary = usize::from(self.stone_checks.iter().any(|s| s.range != 0));
        let total = self.nodes.len()
            + count(|k| matches!(k, NodeKind::NoiseThreshold(_)))
            + count(|k| {
                matches!(
                    k,
                    NodeKind::VerticalGradient(_) | NodeKind::PaperFlatBedrock(_)
                )
            })
            + secondary
            + any(|k| matches!(k, NodeKind::Bandlands))
            + any(|k| matches!(k, NodeKind::Biome { .. } | NodeKind::Temperature))
            + any(|k| matches!(k, NodeKind::Temperature));
        u32::try_from(total).map_err(|_| malformed())
    }
}

/// First-appearance slot of a distinct value.
fn intern<T: PartialEq>(table: &mut Vec<T>, value: T) -> usize {
    if let Some(slot) = table.iter().position(|entry| *entry == value) {
        return slot;
    }
    table.push(value);
    table.len() - 1
}

/// Original `BlockState` codec: `Name` plus optional string `Properties`;
/// unspecified properties take the block's default (strict: unknown names,
/// properties or values are refused rather than leniently defaulted).
fn block_state(value: &JsonValue<'_>) -> Option<BlockStateId> {
    let JsonValueKind::Object(map) = &value.kind else {
        return None;
    };
    if map.keys().any(|k| k != "Name" && k != "Properties") {
        return None;
    }
    let JsonValueKind::String(name) = &map.get("Name")?.kind else {
        return None;
    };
    let mut properties = BTreeMap::new();
    if let Some(props) = map.get("Properties") {
        let JsonValueKind::Object(props) = &props.kind else {
            return None;
        };
        for (key, value) in props {
            let JsonValueKind::String(text) = &value.kind else {
                return None;
            };
            properties.insert(key.as_str(), text.as_str());
        }
    }
    compute_state_id(name, &properties).map(BlockStateId::new)
}
