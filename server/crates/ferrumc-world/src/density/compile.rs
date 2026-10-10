//! Builtin-only definition admission. No runtime sampler is constructed here.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use ferrumc_registry::worldgen::{
    density_function_resources, DensityFunctionResourceId, NoiseResourceId,
    NoiseSettingsResourceId, OriginalDefinition, ResourceDependency,
};

use super::graph::{
    self, Binary, Graph, HolderOrigin, Marker, Node, NodeKind, Point, Rarity, RouterMetadata,
    Shift, Unary, Unit,
};
use super::json::{self, Value, ValueKind};
use super::{
    DensityCompileError, DensityCompileErrorKind as Kind, DensityLimit, DensityTemplate,
    NoiseGeometry, RouterField, RouterTemplate, MAX_DENSITY_EDGES, MAX_DENSITY_SOURCE_BYTES,
    MAX_DENSITY_VERTICES,
};

pub(super) fn density(
    resource: DensityFunctionResourceId,
) -> Result<DensityTemplate, DensityCompileError> {
    let (graph, _) = compile(None)?;
    let root = graph
        .roots
        .get(&resource)
        .copied()
        .ok_or_else(|| error(resource.resource_location(), "$", Kind::ProducerDrift))?;
    let root_stats = graph::measure(&graph.nodes, Some(&[root]));
    Ok(DensityTemplate {
        resource,
        graph: Arc::new(graph),
        root_stats,
    })
}
pub(super) fn router(
    resource: NoiseSettingsResourceId,
) -> Result<RouterTemplate, DensityCompileError> {
    let (graph, metadata) = compile(Some(resource))?;
    let metadata =
        metadata.ok_or_else(|| error(resource.resource_location(), "$", Kind::ProducerDrift))?;
    let roots = graph
        .router_roots
        .ok_or_else(|| error(resource.resource_location(), "$", Kind::ProducerDrift))?;
    let field_stats =
        std::array::from_fn(|index| graph::measure(&graph.nodes, Some(&[roots[index]])));
    Ok(RouterTemplate {
        resource,
        graph: Arc::new(graph),
        metadata,
        field_stats,
    })
}

fn error(resource: &'static str, field: &'static str, kind: Kind) -> DensityCompileError {
    DensityCompileError {
        resource,
        field,
        kind,
        offset: None,
    }
}
fn limit(
    resource: &'static str,
    cap: DensityLimit,
    count: usize,
    maximum: usize,
) -> Result<(), DensityCompileError> {
    if count > maximum {
        return Err(error(
            resource,
            "$",
            Kind::Limit {
                limit: cap,
                count,
                maximum,
            },
        ));
    }
    Ok(())
}

// Typed producer identity, not JSON equality, deduplicates the original sources.
// The complete preflight runs before the first parser/tree/graph allocation.
fn preflight(
    settings: Option<NoiseSettingsResourceId>,
) -> Result<(usize, BTreeSet<NoiseResourceId>), DensityCompileError> {
    let mut bytes = 0;
    let mut noises = BTreeSet::new();
    for id in density_function_resources() {
        source_admit(&mut bytes, id.resource_location(), id.definition())?;
        for dependency in id.definition().dependencies() {
            if let ResourceDependency::Noise(noise) = dependency {
                noises.insert(*noise);
            }
        }
    }
    if let Some(id) = settings {
        source_admit(&mut bytes, id.resource_location(), id.definition())?;
        for dependency in id.definition().dependencies() {
            if let ResourceDependency::Noise(noise) = dependency {
                noises.insert(*noise);
            }
        }
    }
    for &id in &noises {
        source_admit(&mut bytes, id.resource_location(), id.definition())?;
    }
    Ok((bytes, noises))
}
fn source_admit(
    bytes: &mut usize,
    resource: &'static str,
    definition: &OriginalDefinition,
) -> Result<(), DensityCompileError> {
    limit(
        resource,
        DensityLimit::DefinitionBytes,
        definition.json().len(),
        ferrumc_registry::worldgen::MAX_DEFINITION_BYTES,
    )?;
    let count = bytes.checked_add(definition.json().len()).ok_or_else(|| {
        error(
            resource,
            "$",
            Kind::Limit {
                limit: DensityLimit::SourceBytes,
                count: usize::MAX,
                maximum: MAX_DENSITY_SOURCE_BYTES,
            },
        )
    })?;
    limit(
        resource,
        DensityLimit::SourceBytes,
        count,
        MAX_DENSITY_SOURCE_BYTES,
    )?;
    *bytes = count;
    Ok(())
}
fn parse_original(
    definition: &'static OriginalDefinition,
    resource: &'static str,
) -> Result<Value<'static>, DensityCompileError> {
    let (value, stats) = json::parse(definition.json(), resource)?;
    let original = definition.stats();
    if definition.json().len() != original.bytes()
        || stats.nodes != original.nodes()
        || stats.depth != original.maximum_depth()
    {
        return Err(error(resource, "$", Kind::ProducerDrift));
    }
    Ok(value)
}

fn compile(
    settings: Option<NoiseSettingsResourceId>,
) -> Result<(Graph, Option<RouterMetadata>), DensityCompileError> {
    let (source_bytes, source_noises) = preflight(settings)?;
    // Validate the actual original number tokens against the typed producer.
    // This does not create seeded samplers or replace inline-holder identities.
    for &id in &source_noises {
        let value = parse_original(id.definition(), id.resource_location())?;
        check_noise_definition(&value, id)?;
    }
    let mut compiler = Compiler::new();
    for id in density_function_resources() {
        compiler.resource = id.resource_location();
        compiler.dependencies.clear();
        let value = parse_original(id.definition(), id.resource_location())?;
        let root = compiler.direct(&value)?;
        compiler.check_dependencies(id.definition())?;
        compiler.roots.insert(id, root);
    }
    let (router_roots, metadata) = if let Some(id) = settings {
        compiler.resource = id.resource_location();
        compiler.dependencies.clear();
        let value = parse_original(id.definition(), id.resource_location())?;
        let (roots, metadata) = compiler.settings(&value, id.definition().json())?;
        compiler.check_dependencies(id.definition())?;
        (Some(roots), Some(metadata))
    } else {
        (None, None)
    };
    compiler.resolve()?;
    let stats = graph::validate(&mut compiler.nodes)?;
    let noises: BTreeSet<_> = compiler
        .nodes
        .iter()
        .filter_map(|node| node.kind.noise())
        .collect();
    if !noises.is_subset(&source_noises) {
        return Err(compiler.failure("$", Kind::ProducerDrift));
    }
    let graph = Graph {
        nodes: compiler.nodes.into_boxed_slice(),
        roots: compiler.roots,
        router_roots,
        stats,
        source_bytes,
        noises: noises.into_iter().collect(),
    };
    Ok((graph, metadata))
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Dependency {
    Noise(NoiseResourceId),
    Density(DensityFunctionResourceId),
    Type(&'static str),
}
struct Compiler {
    nodes: Vec<Node>,
    roots: BTreeMap<DensityFunctionResourceId, usize>,
    edges: usize,
    resource: &'static str,
    dependencies: BTreeSet<Dependency>,
}
impl Compiler {
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            roots: BTreeMap::new(),
            edges: 0,
            resource: "builtin",
            dependencies: BTreeSet::new(),
        }
    }
    fn failure(&self, field: &'static str, kind: Kind) -> DensityCompileError {
        error(self.resource, field, kind)
    }
    fn add(&mut self, kind: NodeKind) -> Result<usize, DensityCompileError> {
        limit(
            self.resource,
            DensityLimit::Vertices,
            self.nodes.len() + 1,
            MAX_DENSITY_VERTICES,
        )?;
        let edges = self.edges + kind.children();
        limit(self.resource, DensityLimit::Edges, edges, MAX_DENSITY_EDGES)?;
        let index = self.nodes.len();
        self.nodes.push(Node {
            resource: self.resource,
            kind,
            depth: 0,
        });
        self.edges = edges;
        Ok(index)
    }
    fn resolve(&mut self) -> Result<(), DensityCompileError> {
        for node in &mut self.nodes {
            if let NodeKind::Holder {
                origin: HolderOrigin::Reference(id),
                target,
            }
            | NodeKind::Coordinate {
                origin: HolderOrigin::Reference(id),
                target,
            } = &mut node.kind
            {
                *target = self
                    .roots
                    .get(id)
                    .copied()
                    .ok_or_else(|| error(node.resource, "$", Kind::UnknownReference))?;
            }
        }
        Ok(())
    }
    fn check_dependencies(
        &self,
        definition: &OriginalDefinition,
    ) -> Result<(), DensityCompileError> {
        let expected: BTreeSet<_> = definition
            .dependencies()
            .iter()
            .filter_map(|dependency| match dependency {
                ResourceDependency::Noise(id) => Some(Dependency::Noise(*id)),
                ResourceDependency::DensityFunction(id) => Some(Dependency::Density(*id)),
                ResourceDependency::DensityFunctionType(name) => Some(Dependency::Type(name)),
                _ => None,
            })
            .collect();
        if self.dependencies != expected {
            return Err(self.failure("$", Kind::ProducerDrift));
        }
        Ok(())
    }
    fn density_holder(
        &mut self,
        value: &Value<'_>,
        coordinate: bool,
    ) -> Result<usize, DensityCompileError> {
        let (origin, target) = if let ValueKind::String(name) = &value.kind {
            let id = DensityFunctionResourceId::from_name(name)
                .ok_or_else(|| self.failure("$", Kind::UnknownReference))?;
            self.dependencies.insert(Dependency::Density(id));
            (HolderOrigin::Reference(id), usize::MAX)
        } else {
            (HolderOrigin::Direct, self.direct(value)?)
        };
        if coordinate {
            self.add(NodeKind::Coordinate { origin, target })
        } else {
            self.add(NodeKind::Holder { origin, target })
        }
    }
    fn noise(
        &mut self,
        value: &Value<'_>,
        field: &'static str,
    ) -> Result<NoiseResourceId, DensityCompileError> {
        let name = text(value, self.resource, field)?;
        let id = NoiseResourceId::from_name(name)
            .ok_or_else(|| self.failure(field, Kind::UnknownReference))?;
        self.dependencies.insert(Dependency::Noise(id));
        Ok(id)
    }
    fn direct(&mut self, value: &Value<'_>) -> Result<usize, DensityCompileError> {
        if matches!(value.kind, ValueKind::Number(_)) {
            return self.add(NodeKind::Constant(number(
                value,
                self.resource,
                "$",
                -1_000_000.0,
                1_000_000.0,
            )?));
        }
        let object = raw_object(value, self.resource, "$")?;
        let name = text(
            required(object, "type", self.resource)?,
            self.resource,
            "type",
        )?;
        let bare = name.strip_prefix("minecraft:").unwrap_or(name);
        let fields = fields(bare).ok_or_else(|| self.failure("type", Kind::UnknownType))?;
        exact_object(value, fields, self.resource, "$")?;
        let canonical = type_name(bare).ok_or_else(|| self.failure("type", Kind::UnknownType))?;
        self.dependencies.insert(Dependency::Type(canonical));
        let kind = match bare {
            "constant" => {
                NodeKind::Constant(self.scalar(object, "argument", -1_000_000.0, 1_000_000.0)?)
            }
            "add" | "mul" | "min" | "max" => self.binary(object, bare)?,
            "abs" | "square" | "cube" | "half_negative" | "quarter_negative" | "squeeze" => {
                self.mapped(object, bare)?
            }
            "interpolated" | "flat_cache" | "cache_2d" | "cache_once" | "cache_all_in_cell" => {
                self.marker(object, bare)?
            }
            "blend_density" => NodeKind::BlendDensity(self.holder_argument(object, "argument")?),
            "clamp" => NodeKind::Clamp {
                input: self.direct(required(object, "input", self.resource)?)?,
                min: self.scalar(object, "min", -1_000_000.0, 1_000_000.0)?,
                max: self.scalar(object, "max", -1_000_000.0, 1_000_000.0)?,
            },
            "range_choice" => NodeKind::RangeChoice {
                input: self.holder_argument(object, "input")?,
                inside: self.holder_argument(object, "when_in_range")?,
                outside: self.holder_argument(object, "when_out_of_range")?,
                min: self.scalar(object, "min_inclusive", -1_000_000.0, 1_000_000.0)?,
                max: self.scalar(object, "max_exclusive", -1_000_000.0, 1_000_000.0)?,
            },
            "y_clamped_gradient" => NodeKind::Gradient {
                from_y: integer(
                    required(object, "from_y", self.resource)?,
                    self.resource,
                    "from_y",
                    -4064,
                    4062,
                )?,
                to_y: integer(
                    required(object, "to_y", self.resource)?,
                    self.resource,
                    "to_y",
                    -4064,
                    4062,
                )?,
                from_value: self.scalar(object, "from_value", -1_000_000.0, 1_000_000.0)?,
                to_value: self.scalar(object, "to_value", -1_000_000.0, 1_000_000.0)?,
            },
            "noise"
            | "shift"
            | "shift_a"
            | "shift_b"
            | "shifted_noise"
            | "weird_scaled_sampler"
            | "old_blended_noise" => self.noise_node(object, bare)?,
            "spline" => NodeKind::Spline(self.spline(
                required(object, "spline", self.resource)?,
                &mut SplineBudget::default(),
                1,
            )?),
            "end_islands" => NodeKind::Unit(Unit::EndIslands),
            "blend_alpha" => NodeKind::Unit(Unit::BlendAlpha),
            "blend_offset" => NodeKind::Unit(Unit::BlendOffset),
            "beardifier" => NodeKind::Unit(Unit::Beardifier),
            _ => return Err(self.failure("type", Kind::UnknownType)),
        };
        self.add(kind)
    }
    fn scalar(
        &self,
        object: &Object<'_>,
        field: &'static str,
        min: f64,
        max: f64,
    ) -> Result<f64, DensityCompileError> {
        number(
            required(object, field, self.resource)?,
            self.resource,
            field,
            min,
            max,
        )
    }
    fn holder_argument(
        &mut self,
        object: &Object<'_>,
        field: &'static str,
    ) -> Result<usize, DensityCompileError> {
        self.density_holder(required(object, field, self.resource)?, false)
    }
    fn binary(&mut self, object: &Object<'_>, bare: &str) -> Result<NodeKind, DensityCompileError> {
        let left = self.holder_argument(object, "argument1")?;
        let right = self.holder_argument(object, "argument2")?;
        let operation = match bare {
            "add" => Binary::Add,
            "mul" => Binary::Mul,
            "min" => Binary::Min,
            _ => Binary::Max,
        };
        // The real codec wraps both inputs in HolderHolder, so create() cannot
        // use its immediate Constant fast path. The later visitor rebuilds it.
        Ok(NodeKind::Binary {
            operation,
            left,
            right,
        })
    }
    fn mapped(&mut self, object: &Object<'_>, bare: &str) -> Result<NodeKind, DensityCompileError> {
        let input = self.holder_argument(object, "argument")?;
        let operation = match bare {
            "abs" => Unary::Abs,
            "square" => Unary::Square,
            "cube" => Unary::Cube,
            "half_negative" => Unary::HalfNegative,
            "quarter_negative" => Unary::QuarterNegative,
            _ => Unary::Squeeze,
        };
        Ok(NodeKind::Unary { operation, input })
    }
    fn marker(&mut self, object: &Object<'_>, bare: &str) -> Result<NodeKind, DensityCompileError> {
        let input = self.holder_argument(object, "argument")?;
        let marker = match bare {
            "interpolated" => Marker::Interpolated,
            "flat_cache" => Marker::FlatCache,
            "cache_2d" => Marker::Cache2D,
            "cache_once" => Marker::CacheOnce,
            _ => Marker::CacheAllInCell,
        };
        Ok(NodeKind::Marker { marker, input })
    }
    fn noise_node(
        &mut self,
        object: &Object<'_>,
        bare: &str,
    ) -> Result<NodeKind, DensityCompileError> {
        Ok(match bare {
            "noise" => NodeKind::Noise {
                resource: self.noise(required(object, "noise", self.resource)?, "noise")?,
                xz: self.scalar(object, "xz_scale", -f64::MAX, f64::MAX)?,
                y: self.scalar(object, "y_scale", -f64::MAX, f64::MAX)?,
            },
            "shift" | "shift_a" | "shift_b" => NodeKind::Shift {
                shift: match bare {
                    "shift" => Shift::All,
                    "shift_a" => Shift::A,
                    _ => Shift::B,
                },
                resource: self.noise(required(object, "argument", self.resource)?, "argument")?,
            },
            "shifted_noise" => NodeKind::ShiftedNoise {
                shift_x: self.holder_argument(object, "shift_x")?,
                shift_y: self.holder_argument(object, "shift_y")?,
                shift_z: self.holder_argument(object, "shift_z")?,
                resource: self.noise(required(object, "noise", self.resource)?, "noise")?,
                xz: self.scalar(object, "xz_scale", -f64::MAX, f64::MAX)?,
                y: self.scalar(object, "y_scale", -f64::MAX, f64::MAX)?,
            },
            "weird_scaled_sampler" => NodeKind::Weird {
                input: self.holder_argument(object, "input")?,
                resource: self.noise(required(object, "noise", self.resource)?, "noise")?,
                rarity: match text(
                    required(object, "rarity_value_mapper", self.resource)?,
                    self.resource,
                    "rarity_value_mapper",
                )? {
                    "type_1" => Rarity::Type1,
                    "type_2" => Rarity::Type2,
                    _ => return Err(self.failure("rarity_value_mapper", Kind::WrongShape)),
                },
            },
            "old_blended_noise" => NodeKind::Blended {
                xz_scale: self.scalar(object, "xz_scale", 0.001, 1000.0)?,
                y_scale: self.scalar(object, "y_scale", 0.001, 1000.0)?,
                xz_factor: self.scalar(object, "xz_factor", 0.001, 1000.0)?,
                y_factor: self.scalar(object, "y_factor", 0.001, 1000.0)?,
                smear: self.scalar(object, "smear_scale_multiplier", 1.0, 8.0)?,
            },
            _ => return Err(self.failure("type", Kind::UnknownType)),
        })
    }
    fn spline(
        &mut self,
        value: &Value<'_>,
        budget: &mut SplineBudget,
        depth: usize,
    ) -> Result<usize, DensityCompileError> {
        limit(
            self.resource,
            DensityLimit::SplineVertices,
            budget.nodes + 1,
            crate::spline::MAX_SPLINE_NODES,
        )?;
        limit(
            self.resource,
            DensityLimit::SplineDepth,
            depth,
            crate::spline::MAX_SPLINE_DEPTH,
        )?;
        budget.nodes += 1;
        if matches!(value.kind, ValueKind::Number(_)) {
            return self.add(NodeKind::SplineConstant(float(
                value,
                self.resource,
                "spline",
            )?));
        }
        let object = exact_object(value, &["coordinate", "points"], self.resource, "spline")?;
        let coordinate =
            self.density_holder(required(object, "coordinate", self.resource)?, true)?;
        let coordinate_key = match self.nodes[coordinate].kind {
            NodeKind::Coordinate {
                origin: HolderOrigin::Reference(id),
                ..
            } => CoordinateIdentity::Reference(id),
            _ => CoordinateIdentity::Inline(coordinate),
        };
        if !budget.coordinates.contains(&coordinate_key) {
            limit(
                self.resource,
                DensityLimit::SplineCoordinates,
                budget.coordinates.len() + 1,
                crate::spline::MAX_SPLINE_COORDINATES,
            )?;
            budget.coordinates.insert(coordinate_key);
        }
        let values = array(
            required(object, "points", self.resource)?,
            self.resource,
            "points",
        )?;
        if values.is_empty() {
            return Err(self.failure("points", Kind::WrongShape));
        }
        limit(
            self.resource,
            DensityLimit::SplinePointsPerNode,
            values.len(),
            crate::spline::MAX_SPLINE_POINTS_PER_NODE,
        )?;
        limit(
            self.resource,
            DensityLimit::SplinePoints,
            budget.points + values.len(),
            crate::spline::MAX_SPLINE_POINTS,
        )?;
        budget.points += values.len();
        let mut points: Vec<Point> = Vec::with_capacity(values.len());
        for value in values {
            let object = exact_object(
                value,
                &["location", "value", "derivative"],
                self.resource,
                "points",
            )?;
            let location = float(
                required(object, "location", self.resource)?,
                self.resource,
                "location",
            )?;
            let derivative = float(
                required(object, "derivative", self.resource)?,
                self.resource,
                "derivative",
            )?;
            if points
                .last()
                .is_some_and(|point| point.location >= location)
            {
                return Err(self.failure("location", Kind::InvalidSplineOrder));
            }
            let value =
                self.spline(required(object, "value", self.resource)?, budget, depth + 1)?;
            points.push(Point {
                location,
                derivative,
                value,
            });
        }
        self.add(NodeKind::SplineMultipoint {
            coordinate,
            points: points.into_boxed_slice(),
        })
    }
    fn settings(
        &mut self,
        value: &Value<'static>,
        original: &'static str,
    ) -> Result<([usize; 15], RouterMetadata), DensityCompileError> {
        let object = exact_object(
            value,
            &[
                "noise",
                "noise_router",
                "default_block",
                "default_fluid",
                "surface_rule",
                "spawn_target",
                "sea_level",
                "disable_mob_generation",
                "aquifers_enabled",
                "ore_veins_enabled",
                "legacy_random_source",
            ],
            self.resource,
            "$",
        )?;
        let get = |field| required(object, field, self.resource);
        let geometry = geometry(get("noise")?, self.resource)?;
        let router_fields: Vec<_> = RouterField::ALL
            .into_iter()
            .map(RouterField::name)
            .collect();
        let router = exact_object(
            get("noise_router")?,
            &router_fields,
            self.resource,
            "noise_router",
        )?;
        let mut roots = [0usize; 15];
        for field in RouterField::ALL {
            roots[field as usize] =
                self.density_holder(required(router, field.name(), self.resource)?, false)?;
        }
        // Surface noise parameters are dependencies of the retained deferred
        // payload too. This census does not pretend to compile SurfaceRules.
        self.deferred_noise_dependencies(required(object, "surface_rule", self.resource)?)?;
        let metadata = RouterMetadata {
            geometry,
            sea_level: integer(
                required(object, "sea_level", self.resource)?,
                self.resource,
                "sea_level",
                i32::MIN,
                i32::MAX,
            )?,
            algorithm: if boolean(
                required(object, "legacy_random_source", self.resource)?,
                self.resource,
                "legacy_random_source",
            )? {
                crate::random::RandomAlgorithm::Legacy
            } else {
                crate::random::RandomAlgorithm::Xoroshiro128PlusPlus
            },
            aquifers: boolean(
                required(object, "aquifers_enabled", self.resource)?,
                self.resource,
                "aquifers_enabled",
            )?,
            ore_veins: boolean(
                required(object, "ore_veins_enabled", self.resource)?,
                self.resource,
                "ore_veins_enabled",
            )?,
            disable_mobs: boolean(
                required(object, "disable_mob_generation", self.resource)?,
                self.resource,
                "disable_mob_generation",
            )?,
            default_block: &original[required(object, "default_block", self.resource)?
                .span
                .clone()],
            default_fluid: &original[required(object, "default_fluid", self.resource)?
                .span
                .clone()],
            surface_rule: &original[required(object, "surface_rule", self.resource)?
                .span
                .clone()],
            spawn_target: &original[required(object, "spawn_target", self.resource)?
                .span
                .clone()],
        };
        if [
            metadata.default_block,
            metadata.default_fluid,
            metadata.surface_rule,
            metadata.spawn_target,
        ]
        .into_iter()
        .any(str::is_empty)
        {
            return Err(self.failure("$", Kind::ProducerDrift));
        }
        Ok((roots, metadata))
    }
    fn deferred_noise_dependencies(
        &mut self,
        value: &Value<'_>,
    ) -> Result<(), DensityCompileError> {
        match &value.kind {
            ValueKind::Object(object) => {
                if object.get("type").is_some_and(|value| matches!(&value.kind, ValueKind::String(name) if name == "minecraft:noise_threshold")) {
                    self.noise(required(object, "noise", self.resource)?, "noise")?;
                }
                for value in object.values() {
                    self.deferred_noise_dependencies(value)?;
                }
            }
            ValueKind::Array(values) => {
                for value in values {
                    self.deferred_noise_dependencies(value)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

#[derive(Default)]
struct SplineBudget {
    nodes: usize,
    points: usize,
    coordinates: BTreeSet<CoordinateIdentity>,
}
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum CoordinateIdentity {
    Reference(DensityFunctionResourceId),
    Inline(usize),
}

fn fields(name: &str) -> Option<&'static [&'static str]> {
    Some(match name {
        "end_islands" | "blend_alpha" | "blend_offset" | "beardifier" => &["type"],
        "add" | "mul" | "min" | "max" => &["type", "argument1", "argument2"],
        "clamp" => &["type", "input", "min", "max"],
        "range_choice" => &[
            "type",
            "input",
            "min_inclusive",
            "max_exclusive",
            "when_in_range",
            "when_out_of_range",
        ],
        "y_clamped_gradient" => &["type", "from_y", "to_y", "from_value", "to_value"],
        "noise" => &["type", "noise", "xz_scale", "y_scale"],
        "shifted_noise" => &[
            "type", "shift_x", "shift_y", "shift_z", "xz_scale", "y_scale", "noise",
        ],
        "weird_scaled_sampler" => &["type", "input", "noise", "rarity_value_mapper"],
        "old_blended_noise" => &[
            "type",
            "xz_scale",
            "y_scale",
            "xz_factor",
            "y_factor",
            "smear_scale_multiplier",
        ],
        "spline" => &["type", "spline"],
        "constant" | "shift" | "shift_a" | "shift_b" | "abs" | "square" | "cube"
        | "half_negative" | "quarter_negative" | "squeeze" | "interpolated" | "flat_cache"
        | "cache_2d" | "cache_once" | "cache_all_in_cell" | "blend_density" => {
            &["type", "argument"]
        }
        _ => return None,
    })
}
fn type_name(name: &str) -> Option<&'static str> {
    const NAMES: [&str; 32] = [
        "minecraft:constant",
        "minecraft:add",
        "minecraft:mul",
        "minecraft:min",
        "minecraft:max",
        "minecraft:abs",
        "minecraft:square",
        "minecraft:cube",
        "minecraft:half_negative",
        "minecraft:quarter_negative",
        "minecraft:squeeze",
        "minecraft:interpolated",
        "minecraft:flat_cache",
        "minecraft:cache_2d",
        "minecraft:cache_once",
        "minecraft:cache_all_in_cell",
        "minecraft:blend_density",
        "minecraft:clamp",
        "minecraft:range_choice",
        "minecraft:y_clamped_gradient",
        "minecraft:noise",
        "minecraft:shift",
        "minecraft:shift_a",
        "minecraft:shift_b",
        "minecraft:shifted_noise",
        "minecraft:weird_scaled_sampler",
        "minecraft:old_blended_noise",
        "minecraft:spline",
        "minecraft:end_islands",
        "minecraft:blend_alpha",
        "minecraft:blend_offset",
        "minecraft:beardifier",
    ];
    NAMES.into_iter().find(|canonical| canonical[10..] == *name)
}
type Object<'a> = BTreeMap<String, Value<'a>>;
fn raw_object<'a, 'b>(
    value: &'b Value<'a>,
    resource: &'static str,
    field: &'static str,
) -> Result<&'b Object<'a>, DensityCompileError> {
    if let ValueKind::Object(object) = &value.kind {
        Ok(object)
    } else {
        Err(error(resource, field, Kind::WrongShape))
    }
}
fn required<'a, 'b>(
    object: &'b Object<'a>,
    field: &'static str,
    resource: &'static str,
) -> Result<&'b Value<'a>, DensityCompileError> {
    object
        .get(field)
        .ok_or_else(|| error(resource, field, Kind::MissingField))
}
fn exact_object<'a, 'b>(
    value: &'b Value<'a>,
    fields: &[&'static str],
    resource: &'static str,
    field: &'static str,
) -> Result<&'b Object<'a>, DensityCompileError> {
    let object = raw_object(value, resource, field)?;
    for key in object.keys() {
        if !fields.contains(&key.as_str()) {
            return Err(error(resource, field, Kind::UnknownField));
        }
    }
    for &field in fields {
        required(object, field, resource)?;
    }
    Ok(object)
}
fn text<'a>(
    value: &'a Value<'_>,
    resource: &'static str,
    field: &'static str,
) -> Result<&'a str, DensityCompileError> {
    if let ValueKind::String(value) = &value.kind {
        Ok(value)
    } else {
        Err(error(resource, field, Kind::WrongShape))
    }
}
fn array<'a, 'b>(
    value: &'b Value<'a>,
    resource: &'static str,
    field: &'static str,
) -> Result<&'b [Value<'a>], DensityCompileError> {
    if let ValueKind::Array(value) = &value.kind {
        Ok(value)
    } else {
        Err(error(resource, field, Kind::WrongShape))
    }
}
fn number(
    value: &Value<'_>,
    resource: &'static str,
    field: &'static str,
    min: f64,
    max: f64,
) -> Result<f64, DensityCompileError> {
    let ValueKind::Number(token) = value.kind else {
        return Err(error(resource, field, Kind::WrongShape));
    };
    let value = token
        .parse::<f64>()
        .map_err(|_| error(resource, field, Kind::NumericRange))?;
    if !value.is_finite() || value < min || value > max {
        return Err(error(resource, field, Kind::NumericRange));
    }
    Ok(value)
}
fn float(
    value: &Value<'_>,
    resource: &'static str,
    field: &'static str,
) -> Result<f32, DensityCompileError> {
    let ValueKind::Number(token) = value.kind else {
        return Err(error(resource, field, Kind::WrongShape));
    };
    let value = token
        .parse::<f32>()
        .map_err(|_| error(resource, field, Kind::NumericRange))?;
    if !value.is_finite() {
        return Err(error(resource, field, Kind::NumericRange));
    }
    Ok(value)
}
fn integer(
    value: &Value<'_>,
    resource: &'static str,
    field: &'static str,
    min: i32,
    max: i32,
) -> Result<i32, DensityCompileError> {
    let ValueKind::Number(token) = value.kind else {
        return Err(error(resource, field, Kind::WrongShape));
    };
    // Bundled Gson 2.11 NumberLimits admits at most 10000 token characters and
    // strictly less than 10000 absolute BigDecimal scale. Check before any
    // mantissa loop; the original throws NumberFormatException at these caps.
    limit(
        resource,
        DensityLimit::IntegerTokenBytes,
        token.len(),
        10_000,
    )?;
    // JsonOps delegates INT to Gson Number.intValue. Decimal fractions truncate
    // toward zero, and an oversized integer wraps modulo 2^32 BEFORE range
    // validation. Casting a rounded f64 would lose legitimate low integer bits.
    let negative = token.starts_with('-');
    let unsigned = token.strip_prefix('-').unwrap_or(token);
    let (mantissa, exponent) = if let Some(index) = unsigned.find(['e', 'E']) {
        (
            &unsigned[..index],
            unsigned[index + 1..]
                .parse::<i32>()
                .map_err(|_| error(resource, field, Kind::NumericRange))?,
        )
    } else {
        (unsigned, 0)
    };
    let before = i64::try_from(mantissa.find('.').unwrap_or(mantissa.len()))
        .map_err(|_| error(resource, field, Kind::NumericRange))?;
    let fraction = mantissa
        .find('.')
        .map_or(0, |point| mantissa.len() - point - 1);
    let scale = i64::try_from(fraction).map_err(|_| error(resource, field, Kind::NumericRange))?
        - i64::from(exponent);
    if scale.abs() >= 10_000 {
        return Err(error(resource, field, Kind::NumericRange));
    }
    let mut remaining = before + i64::from(exponent);
    let mut bits = 0u32;
    for byte in mantissa.bytes().filter(|byte| *byte != b'.') {
        if remaining <= 0 {
            break;
        }
        bits = bits.wrapping_mul(10).wrapping_add(u32::from(byte - b'0'));
        remaining -= 1;
    }
    if remaining >= 32 {
        bits = 0;
    } else {
        while remaining > 0 {
            bits = bits.wrapping_mul(10);
            remaining -= 1;
        }
    }
    if negative {
        bits = bits.wrapping_neg();
    }
    let result = i32::from_ne_bytes(bits.to_ne_bytes());
    if result < min || result > max {
        return Err(error(resource, field, Kind::NumericRange));
    }
    Ok(result)
}
fn boolean(
    value: &Value<'_>,
    resource: &'static str,
    field: &'static str,
) -> Result<bool, DensityCompileError> {
    if let ValueKind::Bool(value) = value.kind {
        Ok(value)
    } else {
        Err(error(resource, field, Kind::WrongShape))
    }
}
fn geometry(
    value: &Value<'_>,
    resource: &'static str,
) -> Result<NoiseGeometry, DensityCompileError> {
    let object = exact_object(
        value,
        &["min_y", "height", "size_horizontal", "size_vertical"],
        resource,
        "noise",
    )?;
    let min_y = integer(
        required(object, "min_y", resource)?,
        resource,
        "min_y",
        -2032,
        2031,
    )?;
    let height = integer(
        required(object, "height", resource)?,
        resource,
        "height",
        0,
        4064,
    )?;
    let horizontal = integer(
        required(object, "size_horizontal", resource)?,
        resource,
        "size_horizontal",
        1,
        4,
    )?;
    let vertical = integer(
        required(object, "size_vertical", resource)?,
        resource,
        "size_vertical",
        1,
        4,
    )?;
    if min_y % 16 != 0 || height % 16 != 0 || min_y + height > 2032 {
        return Err(error(resource, "noise", Kind::InvalidGeometry));
    }
    Ok(NoiseGeometry {
        min_y,
        height: height as u32,
        horizontal: horizontal as u8,
        vertical: vertical as u8,
    })
}
fn check_noise_definition(
    value: &Value<'_>,
    id: NoiseResourceId,
) -> Result<(), DensityCompileError> {
    let resource = id.resource_location();
    let object = exact_object(value, &["firstOctave", "amplitudes"], resource, "$")?;
    let first = integer(
        required(object, "firstOctave", resource)?,
        resource,
        "firstOctave",
        i32::MIN,
        i32::MAX,
    )?;
    let amplitudes = array(
        required(object, "amplitudes", resource)?,
        resource,
        "amplitudes",
    )?;
    let parameters = id.parameters();
    if first != parameters.first_octave() || amplitudes.len() != parameters.amplitudes().len() {
        return Err(error(resource, "$", Kind::ProducerDrift));
    }
    for (value, &expected) in amplitudes.iter().zip(parameters.amplitudes()) {
        if number(value, resource, "amplitudes", -f64::MAX, f64::MAX)?.to_bits()
            != expected.to_bits()
        {
            return Err(error(resource, "amplitudes", Kind::ProducerDrift));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
