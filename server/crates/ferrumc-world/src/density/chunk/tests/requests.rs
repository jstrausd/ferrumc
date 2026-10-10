//! Bounded declared test leaves and actual-JAR request traces; no production callbacks.
use super::wrappers::{marker, original, probe};
use super::*;
use crate::density::bind::{pair, BoundKind};
use crate::density::chunk::fill::{write, Provider};
use crate::density::graph::{Binary, Marker, NodeKind, Rarity, Unary, Unit};
use serde_json::{json, Value};

pub(in crate::density::chunk) struct Leaf {
    pub root: usize,
    pub label: &'static str,
    pub values: Vec<f64>,
    pub fail_at: Option<usize>,
    pub fail_fill: bool,
    calls: usize,
}
pub(in crate::density::chunk) struct Requests {
    pub leaves: Vec<Leaf>,
    pub trace: Vec<Value>,
    with_chunk: bool,
}
impl Requests {
    pub fn record(&mut self, event: Value) {
        assert!(self.trace.len() < 4096);
        self.trace.push(event);
    }
    pub fn sample(
        &mut self,
        root: usize,
        pos: BlockPos,
        context: &Value,
    ) -> Option<Result<f64, DensitySampleError>> {
        let index = self.leaves.iter().position(|l| l.root == root)?;
        let label = self.leaves[index].label;
        self.record(json!({"kind":"request","case":label,"operation":"compute","context":context}));
        let leaf = &mut self.leaves[index];
        let call = leaf.calls;
        leaf.calls += 1;
        if leaf.fail_at == Some(call) {
            return Some(Err(DensitySampleError {
                resource: label,
                field: "$",
                position: Some(pos),
                batch_index: None,
                kind: DensitySampleErrorKind::InvalidGraph,
            }));
        }
        Some(Ok(
            leaf.values[pos.x().rem_euclid(leaf.values.len() as i32) as usize]
        ))
    }
}
pub(in crate::density::chunk) fn context(state: &ChunkState, pos: BlockPos, own: bool) -> Value {
    let mut value = json!({"own":own,"x":pos.x(),"y":pos.y(),"z":pos.z()});
    if state.requests.as_ref().is_some_and(|r| r.with_chunk) {
        value["interpolationCounter"] = json!(state.scalar_counter);
        value["arrayInterpolationCounter"] = json!(state.array_counter);
        value["arrayIndex"] = json!(state.array_index);
    }
    value
}
pub(in crate::density::chunk) fn fill_declared(
    state: &mut ChunkState,
    root: usize,
    values: &mut [f64],
    work: &mut SampleWork,
) -> Option<Result<(), NoiseChunkError>> {
    let index = state
        .requests
        .as_ref()?
        .leaves
        .iter()
        .position(|l| l.root == root)?;
    let requests = state.requests.as_mut()?;
    let label = requests.leaves[index].label;
    requests.record(
        json!({"kind":"request","case":label,"operation":"fillArray","length":values.len()}),
    );
    let leaf = &requests.leaves[index];
    for (i, slot) in values.iter_mut().enumerate() {
        if i == 1 && leaf.fail_fill {
            return Some(Err(fail(NoiseChunkErrorKind::InvalidGraph)));
        }
        if let Err(error) = write(slot, leaf.values[i % leaf.values.len()], state.start, work) {
            return Some(Err(error));
        }
    }
    Some(Ok(()))
}
fn fresh(with_chunk: bool) -> NoiseChunkSession {
    let mut s = probe(vec![(original(NodeKind::Constant(0.0)), "unused")], 0);
    s.state.requests = Some(Requests {
        leaves: Vec::new(),
        trace: Vec::new(),
        with_chunk,
    });
    s
}
fn node(s: &mut NoiseChunkSession, kind: BoundKind) -> usize {
    let root = s.view.nodes.len();
    let mut n = s.view.nodes[0].clone();
    n.kind = kind;
    n.resource = "matrix";
    n.bounds = pair(f64::NEG_INFINITY, f64::INFINITY);
    s.view.nodes.push(n);
    s.state.wrapper_nodes.push(None);
    root
}
fn leaf(
    s: &mut NoiseChunkSession,
    label: &'static str,
    values: &[f64],
    bounds: (f64, f64),
) -> usize {
    assert!(!values.is_empty() && values.len() <= 128);
    let root = node(s, original(NodeKind::Constant(0.0)));
    s.view.nodes[root].bounds = pair(bounds.0, bounds.1);
    let requests = s.state.requests.as_mut().unwrap();
    assert!(requests.leaves.len() < 16);
    requests.leaves.push(Leaf {
        root,
        label,
        values: values.to_vec(),
        fail_at: None,
        fail_fill: false,
        calls: 0,
    });
    root
}
fn controls(s: &mut NoiseChunkSession, root: usize, fail_at: Option<usize>, fail_fill: bool) {
    let l = s
        .state
        .requests
        .as_mut()
        .unwrap()
        .leaves
        .iter_mut()
        .find(|l| l.root == root)
        .unwrap();
    l.fail_at = fail_at;
    l.fail_fill = fail_fill;
}
fn take_trace(s: &mut NoiseChunkSession) -> Vec<Value> {
    std::mem::take(&mut s.state.requests.as_mut().unwrap().trace)
}
fn bits(value: &Value) -> u64 {
    u64::from_str_radix(value.as_str().unwrap(), 16).unwrap()
}
fn rows() -> Vec<Value> {
    super::oracle::records()
}
fn find<'a>(rows: &'a [Value], kind: &str, case: &str) -> &'a Value {
    rows.iter()
        .find(|r| r["kind"] == kind && r["case"] == case)
        .unwrap()
}
fn compare_bits(values: &[f64], expected: &Value, case: &str) {
    let expected = expected.as_array().unwrap();
    assert_eq!(values.len(), expected.len(), "{case}");
    for (i, (v, e)) in values.iter().zip(expected).enumerate() {
        assert_eq!(v.to_bits(), bits(e), "{case} index{i}");
    }
}
fn check_state(s: &NoiseChunkSession, row: &Value) {
    if row.get("state").is_some() {
        assert_eq!(
            row["state"],
            json!({"interpolationCounter":s.state.scalar_counter,"arrayInterpolationCounter":s.state.array_counter,"arrayIndex":s.state.array_index,"interpolating":s.state.interpolating,"fillingCell":s.state.filling_cell,"inCellX":s.state.in_x,"inCellY":s.state.in_y,"inCellZ":s.state.in_z}),
            "{} state",
            row["case"]
        );
    }
}
fn scalar(s: &mut NoiseChunkSession, root: usize, row: &Value, pos: BlockPos, own: bool) {
    let result = s
        .state
        .sample(&s.data, &s.view, root, pos, own, &mut s.work);
    assert_eq!(
        result.is_err(),
        row.get("error").is_some(),
        "{} error",
        row["case"]
    );
    if let Ok(value) = result {
        assert_eq!(
            value.to_bits(),
            bits(&row["result_bits"]),
            "{} bits",
            row["case"]
        );
    }
    assert_eq!(
        take_trace(s),
        *row["trace"].as_array().unwrap(),
        "{} trace",
        row["case"]
    );
    check_state(s, row);
}
fn array(
    s: &mut NoiseChunkSession,
    root: usize,
    row: &Value,
    mut values: Vec<f64>,
    provider: Provider,
) {
    let result = s
        .state
        .fill(&s.data, &s.view, root, &mut values, provider, &mut s.work);
    assert_eq!(
        result.is_err(),
        row.get("error").is_some(),
        "{} error",
        row["case"]
    );
    if let Some(expected) = row.get("array_bits") {
        compare_bits(&values, expected, row["case"].as_str().unwrap());
    }
    let expected = row["trace"].as_array().unwrap();
    let mut trace = take_trace(s);
    for event in expected.iter().filter(|r| r["kind"] == "array-prefix") {
        compare_bits(&values, &event["bits"], row["case"].as_str().unwrap());
        trace.push(json!({"kind":"array-prefix","case":"actual","bits":values.iter().map(|v|format!("{:016x}",v.to_bits())).collect::<Vec<_>>() }));
    }
    assert_eq!(trace, *expected, "{} trace", row["case"]);
    check_state(s, row);
}
mod arrays;
mod providers;
mod wrappers;
