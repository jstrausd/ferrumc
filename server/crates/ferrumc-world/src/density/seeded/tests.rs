use std::collections::BTreeMap;

use ferrumc_registry::worldgen::NoiseResourceId;
use serde_json::Value;

use super::*;
use crate::density::bind::{BoundKind, BoundView};
use crate::density::graph::{Binary, HolderOrigin, Marker, NodeKind, Rarity, Shift, Unary, Unit};

fn bits64(value: &Value) -> u64 {
    u64::from_str_radix(value.as_str().unwrap(), 16).unwrap()
}
fn bits32(value: &Value) -> u32 {
    u32::from_str_radix(value.as_str().unwrap(), 16).unwrap()
}
fn position(record: &Value) -> BlockPos {
    let xyz = record["position"].as_array().unwrap();
    BlockPos::new(
        xyz[0].as_i64().unwrap() as i32,
        xyz[1].as_i64().unwrap() as i32,
        xyz[2].as_i64().unwrap() as i32,
    )
}
fn compare_bounds(actual: DensityBounds, expected: &Value, context: &str) {
    assert_eq!(
        actual.min_value().to_bits(),
        bits64(&expected[0]),
        "min {context}"
    );
    assert_eq!(
        actual.max_value().to_bits(),
        bits64(&expected[1]),
        "max {context}"
    );
}
fn compare_value(actual: Result<f64, DensitySampleError>, expected: &Value, context: &str) {
    let expected = bits64(expected);
    if f64::from_bits(expected).is_finite() {
        assert_eq!(
            actual
                .unwrap_or_else(|error| panic!("{context}: {error:?}"))
                .to_bits(),
            expected,
            "{context}"
        );
    } else {
        assert!(
            matches!(
                actual.unwrap_err().kind(),
                DensitySampleErrorKind::NonFiniteDensity { .. }
                    | DensitySampleErrorKind::NonFiniteCoordinate { .. }
                    | DensitySampleErrorKind::NonFiniteSpline { .. }
            ),
            "{context}"
        );
    }
}

fn probe_node(
    kind: BoundKind,
    label: &'static str,
    origin: crate::density::bind::SourceOrigin,
) -> crate::density::bind::BoundNode {
    let bounds = match kind {
        BoundKind::Original(NodeKind::Constant(value)) => crate::density::bind::pair(value, value),
        _ => crate::density::bind::pair(f64::NEG_INFINITY, f64::INFINITY),
    };
    crate::density::bind::BoundNode {
        origin,
        resource: label,
        kind,
        bounds,
        noise: None,
        blended: None,
        spline: None,
    }
}
fn probe_router(kinds: Vec<(BoundKind, &'static str)>, root: usize) -> SeededRouter {
    use crate::density::bind::VisitorPass;
    let mut router =
        RouterTemplate::compile_builtin(NoiseSettingsResourceId::from_name("overworld").unwrap())
            .unwrap()
            .bind_seed(WorldSeed::new(0))
            .unwrap();
    let data = Arc::get_mut(&mut router.data).unwrap();
    let origin = data.router.nodes[0].origin;
    data.router = BoundView {
        pass: VisitorPass::SeedRouter,
        nodes: kinds
            .into_iter()
            .map(|(kind, label)| probe_node(kind, label, origin))
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    };
    data.fields = [root; 15];
    data.climate_fields = [root; 6];
    router
}
fn original(kind: NodeKind) -> BoundKind {
    BoundKind::Original(kind)
}
fn evaluate_probe(
    router: &SeededRouter,
    root: usize,
    limit: usize,
) -> (Result<f64, DensitySampleError>, SampleWork) {
    let mut work = SampleWork::new(limit);
    work.enable_density_trace(4096);
    let result = evaluate::sample(
        &router.data,
        &router.data.router,
        root,
        BlockPos::new(0, 1, 0),
        &mut work,
        "probe",
    );
    (result, work)
}

mod domain;
mod oracle;
mod origin;
mod projection;
mod trace;
mod work;
