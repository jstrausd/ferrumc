//! Schema-directed builtin reference admission, following the original codecs.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{value::RawValue, Value};

use super::{
    array, boolean, canonical_name, integer, invalid, number, object, require, text,
    BlockProperties, BuildError, Dependency, Kind,
};

const MAX_OCCURRENCES: usize = 2048;
const MAX_DEPENDENCIES: usize = 512;
const NOISE_VALUE: f64 = 1_000_000.0;
const MIN_Y: i64 = -2032;
const MAX_Y: i64 = 2031;

pub(super) fn noise(value: &Value) -> Result<(i32, Vec<f64>), BuildError> {
    let object = object(value, &["firstOctave", "amplitudes"])?;
    let octave = i32::try_from(integer(&object["firstOctave"], -16, -1)?)
        .map_err(|_| invalid("firstOctave conversion"))?;
    let amplitudes = array(&object["amplitudes"])?;
    require(
        (1..=16).contains(&amplitudes.len()),
        "builtin amplitude length outside 1..16",
    )?;
    let amplitudes: Vec<f64> = amplitudes
        .iter()
        .map(|value| number(value, 0.0, 2.0))
        .collect::<Result<_, _>>()?;
    require(
        amplitudes.iter().any(|value| *value != 0.0),
        "builtin noise has no nonzero amplitude",
    )?;
    Ok((octave, amplitudes))
}

pub(super) struct Checker<'a> {
    noise: &'a BTreeMap<String, &'a RawValue>,
    density: &'a BTreeMap<String, &'a RawValue>,
    biomes: &'a BTreeMap<String, &'a RawValue>,
    blocks: &'a BlockProperties,
    pub occurrences: usize,
    pub dependencies: BTreeSet<Dependency>,
}
impl<'a> Checker<'a> {
    pub fn new(
        noise: &'a BTreeMap<String, &'a RawValue>,
        density: &'a BTreeMap<String, &'a RawValue>,
        biomes: &'a BTreeMap<String, &'a RawValue>,
        blocks: &'a BlockProperties,
    ) -> Self {
        Self {
            noise,
            density,
            biomes,
            blocks,
            occurrences: 0,
            dependencies: BTreeSet::new(),
        }
    }
    fn reference(&mut self, kind: Kind, value: &Value) -> Result<(), BuildError> {
        let name = text(value)?;
        require(
            canonical_name(name),
            "malformed/foreign reference name or unsupported tag",
        )?;
        match kind {
            Kind::Noise => require(self.noise.contains_key(name), "unknown noise reference")?,
            Kind::DensityFunction => {
                require(self.density.contains_key(name), "unknown density reference")?;
            }
            Kind::Biome => require(self.biomes.contains_key(name), "unknown biome dependency")?,
            Kind::Block => require(self.blocks.contains_key(name), "unknown block dependency")?,
            _ => {}
        }
        // Charge before allocating an owned dependency. Whole JSON is bounded too.
        if self.occurrences == MAX_OCCURRENCES {
            return Err(BuildError::Bounds("dependency occurrences"));
        }
        self.occurrences += 1;
        if self.dependencies.len() == MAX_DEPENDENCIES
            && !self
                .dependencies
                .iter()
                .any(|entry| entry.kind == kind && entry.name == name)
        {
            return Err(BuildError::Bounds("distinct dependencies"));
        }
        self.dependencies.insert(Dependency {
            kind,
            name: name.to_owned(),
        });
        Ok(())
    }
    /// Holder-helper string references and Directcodec inline data stay separate.
    pub fn density(&mut self, value: &Value, allow_holder: bool) -> Result<(), BuildError> {
        if value.is_number() {
            number(value, -NOISE_VALUE, NOISE_VALUE)?;
            return Ok(());
        }
        if value.is_string() {
            require(allow_holder, "Direct density codec refuses holder strings")?;
            return self.reference(Kind::DensityFunction, value);
        }
        let typ = text(
            value
                .get("type")
                .ok_or_else(|| invalid("density node missing type"))?,
        )?;
        let bare = typ
            .strip_prefix("minecraft:")
            .ok_or_else(|| invalid("foreign density type"))?;
        let node = object(value, density_fields(bare)?)?;
        self.reference(Kind::DensityFunctionType, &node["type"])?;
        match bare {
            "blend_alpha" | "blend_offset" | "end_islands" => {}
            "shift_a" | "shift_b" => self.reference(Kind::Noise, &node["argument"])?,
            "add" | "mul" | "min" | "max" => {
                self.density(&node["argument1"], true)?;
                self.density(&node["argument2"], true)?;
            }
            "clamp" => {
                self.density(&node["input"], false)?;
                number(&node["min"], -NOISE_VALUE, NOISE_VALUE)?;
                number(&node["max"], -NOISE_VALUE, NOISE_VALUE)?;
            }
            "noise" => {
                self.reference(Kind::Noise, &node["noise"])?;
                number(&node["xz_scale"], -f64::MAX, f64::MAX)?;
                number(&node["y_scale"], -f64::MAX, f64::MAX)?;
            }
            "shifted_noise" => {
                for field in ["shift_x", "shift_y", "shift_z"] {
                    self.density(&node[field], true)?;
                }
                self.reference(Kind::Noise, &node["noise"])?;
                number(&node["xz_scale"], -f64::MAX, f64::MAX)?;
                number(&node["y_scale"], -f64::MAX, f64::MAX)?;
            }
            "weird_scaled_sampler" => {
                self.density(&node["input"], true)?;
                self.reference(Kind::Noise, &node["noise"])?;
                require(
                    matches!(text(&node["rarity_value_mapper"])?, "type_1" | "type_2"),
                    "unknown rarity mapper",
                )?;
            }
            "range_choice" => {
                for field in ["input", "when_in_range", "when_out_of_range"] {
                    self.density(&node[field], true)?;
                }
                for field in ["min_inclusive", "max_exclusive"] {
                    number(&node[field], -NOISE_VALUE, NOISE_VALUE)?;
                }
            }
            "y_clamped_gradient" => {
                for field in ["from_y", "to_y"] {
                    integer(&node[field], MIN_Y * 2, MAX_Y * 2)?;
                }
                for field in ["from_value", "to_value"] {
                    number(&node[field], -NOISE_VALUE, NOISE_VALUE)?;
                }
            }
            "old_blended_noise" => {
                for field in ["xz_scale", "y_scale", "xz_factor", "y_factor"] {
                    number(&node[field], 0.001, 1000.0)?;
                }
                number(&node["smear_scale_multiplier"], 1.0, 8.0)?;
            }
            "spline" => self.spline(&node["spline"])?,
            _ => self.density(&node["argument"], true)?,
        }
        Ok(())
    }
    fn spline(&mut self, value: &Value) -> Result<(), BuildError> {
        if value.is_number() {
            float(value)?;
            return Ok(());
        }
        let node = object(value, &["coordinate", "points"])?;
        self.density(&node["coordinate"], true)?;
        let points = array(&node["points"])?;
        require(!points.is_empty(), "empty spline points")?;
        let mut previous = None;
        for point in points {
            let point = object(point, &["location", "value", "derivative"])?;
            let location = float(&point["location"])?;
            require(
                previous.is_none_or(|last| location > last),
                "spline locations not strictly increasing as f32",
            )?;
            previous = Some(location);
            float(&point["derivative"])?;
            self.spline(&point["value"])?;
        }
        Ok(())
    }
    fn block(&mut self, value: &Value) -> Result<(), BuildError> {
        let node = if value.get("Properties").is_some() {
            object(value, &["Name", "Properties"])?
        } else {
            object(value, &["Name"])?
        };
        self.reference(Kind::Block, &node["Name"])?;
        if let Some(properties) = node.get("Properties") {
            let properties = properties
                .as_object()
                .ok_or_else(|| invalid("block properties must be an object"))?;
            let domain = &self.blocks[text(&node["Name"])?];
            for (key, value) in properties {
                require(
                    domain.get(key).is_some_and(|allowed| {
                        value
                            .as_str()
                            .is_some_and(|v| allowed.iter().any(|entry| entry == v))
                    }),
                    "unknown block property/value",
                )?;
            }
        }
        Ok(())
    }
    fn surface_rule(&mut self, value: &Value) -> Result<(), BuildError> {
        let typ = text(
            value
                .get("type")
                .ok_or_else(|| invalid("surface rule missing type"))?,
        )?;
        let fields: &[&str] = match typ {
            "minecraft:bandlands" => &["type"],
            "minecraft:block" => &["type", "result_state"],
            "minecraft:sequence" => &["type", "sequence"],
            "minecraft:condition" => &["type", "if_true", "then_run"],
            _ => return Err(invalid("unknown builtin surface rule form")),
        };
        let node = object(value, fields)?;
        self.reference(Kind::SurfaceRuleType, &node["type"])?;
        match typ {
            "minecraft:block" => self.block(&node["result_state"])?,
            "minecraft:sequence" => {
                for child in array(&node["sequence"])? {
                    self.surface_rule(child)?;
                }
            }
            "minecraft:condition" => {
                self.condition(&node["if_true"])?;
                self.surface_rule(&node["then_run"])?;
            }
            _ => {}
        }
        Ok(())
    }
    fn condition(&mut self, value: &Value) -> Result<(), BuildError> {
        let typ = text(
            value
                .get("type")
                .ok_or_else(|| invalid("surface condition missing type"))?,
        )?;
        let fields: &[&str] = match typ {
            "minecraft:above_preliminary_surface"
            | "minecraft:hole"
            | "minecraft:steep"
            | "minecraft:temperature" => &["type"],
            "minecraft:biome" => &["type", "biome_is"],
            "minecraft:noise_threshold" => &["type", "noise", "min_threshold", "max_threshold"],
            "minecraft:not" => &["type", "invert"],
            "minecraft:stone_depth" => &[
                "type",
                "offset",
                "add_surface_depth",
                "secondary_depth_range",
                "surface_type",
            ],
            "minecraft:vertical_gradient" => &[
                "type",
                "random_name",
                "true_at_and_below",
                "false_at_and_above",
            ],
            "minecraft:water" => &[
                "type",
                "offset",
                "surface_depth_multiplier",
                "add_stone_depth",
            ],
            "minecraft:y_above" => &[
                "type",
                "anchor",
                "surface_depth_multiplier",
                "add_stone_depth",
            ],
            _ => return Err(invalid("unknown builtin surface condition form")),
        };
        let node = object(value, fields)?;
        self.reference(Kind::SurfaceConditionType, &node["type"])?;
        match typ {
            "minecraft:biome" => {
                for biome in array(&node["biome_is"])? {
                    self.reference(Kind::Biome, biome)?;
                }
            }
            "minecraft:noise_threshold" => {
                self.reference(Kind::Noise, &node["noise"])?;
                number(&node["min_threshold"], -f64::MAX, f64::MAX)?;
                number(&node["max_threshold"], -f64::MAX, f64::MAX)?;
            }
            "minecraft:not" => self.condition(&node["invert"])?,
            "minecraft:stone_depth" => {
                for field in ["offset", "secondary_depth_range"] {
                    integer(&node[field], i64::from(i32::MIN), i64::from(i32::MAX))?;
                }
                boolean(&node["add_surface_depth"])?;
                require(
                    matches!(text(&node["surface_type"])?, "floor" | "ceiling"),
                    "unknown cave surface enum",
                )?;
            }
            "minecraft:vertical_gradient" => {
                self.reference(Kind::RandomName, &node["random_name"])?;
                anchor(&node["true_at_and_below"])?;
                anchor(&node["false_at_and_above"])?;
            }
            "minecraft:water" | "minecraft:y_above" => {
                if typ == "minecraft:water" {
                    integer(&node["offset"], i64::from(i32::MIN), i64::from(i32::MAX))?;
                } else {
                    anchor(&node["anchor"])?;
                }
                integer(&node["surface_depth_multiplier"], -20, 20)?;
                boolean(&node["add_stone_depth"])?;
            }
            _ => {}
        }
        Ok(())
    }
    pub fn settings(&mut self, value: &Value) -> Result<(), BuildError> {
        const ROUTER: &[&str] = &[
            "barrier",
            "fluid_level_floodedness",
            "fluid_level_spread",
            "lava",
            "temperature",
            "vegetation",
            "continents",
            "erosion",
            "depth",
            "ridges",
            "initial_density_without_jaggedness",
            "final_density",
            "vein_toggle",
            "vein_ridged",
            "vein_gap",
        ];
        let node = object(
            value,
            &[
                "noise",
                "default_block",
                "default_fluid",
                "noise_router",
                "surface_rule",
                "spawn_target",
                "sea_level",
                "disable_mob_generation",
                "aquifers_enabled",
                "ore_veins_enabled",
                "legacy_random_source",
            ],
        )?;
        let geometry = object(
            &node["noise"],
            &["min_y", "height", "size_horizontal", "size_vertical"],
        )?;
        let min_y = integer(&geometry["min_y"], MIN_Y, MAX_Y)?;
        let height = integer(&geometry["height"], 0, MAX_Y - MIN_Y + 1)?;
        require(
            min_y % 16 == 0 && height % 16 == 0 && min_y + height <= MAX_Y + 1,
            "invalid noise height geometry",
        )?;
        integer(&geometry["size_horizontal"], 1, 4)?;
        integer(&geometry["size_vertical"], 1, 4)?;
        self.block(&node["default_block"])?;
        self.block(&node["default_fluid"])?;
        let router = object(&node["noise_router"], ROUTER)?;
        for field in ROUTER {
            self.density(&router[*field], true)?;
        }
        self.surface_rule(&node["surface_rule"])?;
        for target in array(&node["spawn_target"])? {
            climate(target)?;
        }
        integer(&node["sea_level"], i64::from(i32::MIN), i64::from(i32::MAX))?;
        for field in [
            "disable_mob_generation",
            "aquifers_enabled",
            "ore_veins_enabled",
            "legacy_random_source",
        ] {
            boolean(&node[field])?;
        }
        Ok(())
    }
}

fn density_fields(bare: &str) -> Result<&'static [&'static str], BuildError> {
    match bare {
        "blend_alpha" | "blend_offset" | "end_islands" => Ok(&["type"]),
        "abs" | "square" | "cube" | "half_negative" | "quarter_negative" | "squeeze"
        | "interpolated" | "flat_cache" | "cache_2d" | "cache_once" | "blend_density"
        | "shift_a" | "shift_b" => Ok(&["type", "argument"]),
        "add" | "mul" | "min" | "max" => Ok(&["type", "argument1", "argument2"]),
        "clamp" => Ok(&["type", "input", "min", "max"]),
        "noise" => Ok(&["type", "noise", "xz_scale", "y_scale"]),
        "shifted_noise" => Ok(&[
            "type", "shift_x", "shift_y", "shift_z", "xz_scale", "y_scale", "noise",
        ]),
        "weird_scaled_sampler" => Ok(&["type", "input", "noise", "rarity_value_mapper"]),
        "range_choice" => Ok(&[
            "type",
            "input",
            "min_inclusive",
            "max_exclusive",
            "when_in_range",
            "when_out_of_range",
        ]),
        "y_clamped_gradient" => Ok(&["type", "from_y", "to_y", "from_value", "to_value"]),
        "old_blended_noise" => Ok(&[
            "type",
            "xz_scale",
            "y_scale",
            "xz_factor",
            "y_factor",
            "smear_scale_multiplier",
        ]),
        "spline" => Ok(&["type", "spline"]),
        _ => Err(invalid("unknown builtin density form")),
    }
}

fn float(value: &Value) -> Result<f32, BuildError> {
    let value = number(value, -f64::from(f32::MAX), f64::from(f32::MAX))? as f32;
    require(value.is_finite(), "non-finite float codec value")?;
    Ok(value)
}
fn anchor(value: &Value) -> Result<(), BuildError> {
    let node = value
        .as_object()
        .ok_or_else(|| invalid("anchor must be an object"))?;
    require(node.len() == 1, "anchor must have exactly one variant")?;
    let (key, value) = node.iter().next().ok_or_else(|| invalid("empty anchor"))?;
    require(
        matches!(key.as_str(), "absolute" | "above_bottom" | "below_top"),
        "unknown anchor variant",
    )?;
    integer(value, MIN_Y, MAX_Y)?;
    Ok(())
}
fn climate(value: &Value) -> Result<(), BuildError> {
    let node = object(
        value,
        &[
            "temperature",
            "humidity",
            "continentalness",
            "erosion",
            "depth",
            "weirdness",
            "offset",
        ],
    )?;
    for field in [
        "temperature",
        "humidity",
        "continentalness",
        "erosion",
        "depth",
        "weirdness",
    ] {
        if node[field].is_number() {
            number(&node[field], -2.0, 2.0)?;
        } else {
            let interval = array(&node[field])?;
            require(interval.len() == 2, "climate interval must have two ends")?;
            let min = number(&interval[0], -2.0, 2.0)?;
            let max = number(&interval[1], -2.0, 2.0)?;
            require(min <= max, "inverted climate interval")?;
        }
    }
    number(&node["offset"], 0.0, 1.0)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Checker, Kind, MAX_DEPENDENCIES, MAX_OCCURRENCES};
    use std::collections::BTreeMap;

    #[test]
    fn dependency_occurrences_and_distinct_bounds_precede_insertion() {
        let empty = BTreeMap::new();
        let blocks = BTreeMap::new();
        let mut checker = Checker::new(&empty, &empty, &empty, &blocks);
        let name = serde_json::json!("minecraft:abs");
        for _ in 0..MAX_OCCURRENCES {
            checker.reference(Kind::DensityFunctionType, &name).unwrap();
        }
        assert_eq!(checker.dependencies.len(), 1);
        assert!(checker.reference(Kind::DensityFunctionType, &name).is_err());
        assert_eq!(checker.occurrences, MAX_OCCURRENCES);
        assert_eq!(checker.dependencies.len(), 1);
        let mut checker = Checker::new(&empty, &empty, &empty, &blocks);
        for i in 0..MAX_DEPENDENCIES {
            checker
                .reference(
                    Kind::RandomName,
                    &serde_json::json!(format!("minecraft:random{i}")),
                )
                .unwrap();
        }
        assert_eq!(checker.dependencies.len(), MAX_DEPENDENCIES);
        assert!(checker
            .reference(Kind::RandomName, &serde_json::json!("minecraft:one_more"))
            .is_err());
        assert_eq!(checker.dependencies.len(), MAX_DEPENDENCIES);
        checker
            .reference(Kind::RandomName, &serde_json::json!("minecraft:random0"))
            .unwrap();
    }
}
