//! Optional NaturalBinding-only immutable cold graph and wrapper recipe.
//!
//! No wrapper contents, sampled position, counters or work meter are retained.
use std::{mem::size_of, sync::Arc};

use ferrumc_math::{BlockPos, ChunkPos};

use super::state::{Cache, ChunkState, Lifecycle};
use super::{
    admit_chunk, core_charge, NoiseChunkSession, NoiseChunkWorkBudget, SurfaceCache,
    MAX_NOISE_CHUNK_WORK, MAX_SURFACE_COLUMNS,
};
use crate::density::bind::{BoundKind, BoundNode, BoundRouter, BoundSpline, BoundView};
use crate::density::graph::{Marker, NodeKind, Point};
use crate::density::{NoiseChunkError, SeededRouter};
use crate::sample_work::SampleWork;

const PLAN_BYTES: usize = 512 * 1024;
const COMPILE_BYTES: usize = 8 * 1024 * 1024;
// Closed natural geometry, original 128-wrapper/96-depth ceilings and original
// 2 MiB graph/4 MiB cold censuses. Includes two complete stages, both surface
// caches and two full plan-sized graph copies. Shared router/allocator excluded.
const COMPILE_CENSUS: usize = 2
    * (2 * 1024 * 1024
        + 128 * 2 * (5 * 49) * 8
        + 96 * 128 * 8
        + MAX_SURFACE_COLUMNS * 12
        + size_of::<NoiseChunkSession>())
    + 2 * PLAN_BYTES;

struct Recipe {
    marker: Marker,
    input: usize,
    node: usize,
}
pub(crate) struct NaturalNoisePlan {
    data: Arc<BoundRouter>,
    view: BoundView,
    recipes: Vec<Recipe>,
    fields: [usize; 15],
    material: usize,
    climate: [usize; 6],
    static_work: usize,
    graph_bytes: usize,
    scratch_bytes: usize,
    scratch_depth: usize,
    scratch_len: usize,
    #[cfg(test)]
    wrap_entries: usize,
}
impl NaturalNoisePlan {
    fn native_bytes(&self) -> usize {
        size_of::<Self>()
            + self.view.nodes.capacity() * size_of::<BoundNode>()
            + self.recipes.capacity() * size_of::<Recipe>()
            + self.view.nodes.iter().map(node_allocations).sum::<usize>()
    }
    fn compile(data: Arc<BoundRouter>) -> Option<Self> {
        let geometry = data.metadata.geometry;
        if geometry.min_y() != -64
            || geometry.height() != 384
            || geometry.cell_width() != 4
            || geometry.cell_height() != 8
            || COMPILE_CENSUS > COMPILE_BYTES
        {
            return None;
        }
        let original = NoiseChunkSession::new(
            Arc::clone(&data),
            ChunkPos::ORIGIN,
            NoiseChunkWorkBudget::new(MAX_NOISE_CHUNK_WORK).ok()?,
        )
        .ok()?;
        if !topological(&original.view) {
            return None;
        }
        let mut recipes = Vec::new();
        recipes
            .try_reserve_exact(original.state.wrappers.len().checked_sub(2)?)
            .ok()?;
        for wrapper in &original.state.wrappers[2..] {
            let BoundKind::Original(NodeKind::Marker { marker, input }) =
                original.view.nodes.get(wrapper.node)?.kind
            else {
                return None;
            };
            if input != wrapper.input || input >= wrapper.node {
                return None;
            }
            recipes.push(Recipe {
                marker,
                input,
                node: wrapper.node,
            });
        }
        let scratch_len = original.state.scratch.first()?.len();
        if original
            .state
            .scratch
            .iter()
            .any(|v| v.len() != scratch_len)
        {
            return None;
        }
        let mut plan = Self {
            data,
            view: clone_view(&original.view)?,
            recipes,
            fields: original.fields,
            material: original.material,
            climate: original.climate,
            static_work: 0,
            graph_bytes: original.state.graph_bytes,
            scratch_bytes: original.state.scratch_bytes,
            scratch_depth: original.state.scratch.len(),
            scratch_len,
            #[cfg(test)]
            wrap_entries: original.state.wrap_entries,
        };
        if plan.native_bytes() > PLAN_BYTES {
            return None;
        }
        let (state, work) = plan.rebuild(ChunkPos::ORIGIN)?;
        if !original.state.plan_equivalent(&state) {
            return None;
        }
        plan.static_work = original.work.used().checked_sub(work.used())?;
        Some(plan)
    }
    fn rebuild(&self, chunk: ChunkPos) -> Option<(ChunkState, SampleWork)> {
        admit_chunk(chunk).ok()?;
        let mut work = SampleWork::new(MAX_NOISE_CHUNK_WORK as usize);
        let mut state =
            ChunkState::new(&self.data, chunk, self.data.metadata.geometry, &mut work).ok()?;
        state.wrapper_nodes.resize(self.view.nodes.len(), None);
        for recipe in &self.recipes {
            let id = state.add_wrapper(recipe.marker, recipe.input).ok()?;
            if matches!(recipe.marker, Marker::FlatCache) {
                // Original X-outer/Z-inner values are position-dependent. Never
                // copy a successful origin's arrays into another constructor.
                for x in 0..5 {
                    for z in 0..5 {
                        let pos =
                            BlockPos::new(state.origin.x() + x * 4, 0, state.origin.z() + z * 4);
                        let value = state
                            .sample(&self.data, &self.view, recipe.input, pos, false, &mut work)
                            .ok()?;
                        core_charge(&mut work).ok()?;
                        let Cache::Flat(values) = &mut state.wrappers[id].cache else {
                            return None;
                        };
                        values[(x * 5 + z) as usize] = value;
                    }
                }
            }
            // This wrapper becomes visible only after its original flat fill.
            state.wrapper_nodes[recipe.node] = Some(id);
            state.wrappers[id].node = recipe.node;
        }
        state.scratch = (0..self.scratch_depth)
            .map(|_| vec![0.0; self.scratch_len])
            .collect();
        state.graph_bytes = self.graph_bytes;
        state.scratch_bytes = self.scratch_bytes;
        #[cfg(test)]
        {
            state.wrap_entries = self.wrap_entries;
        }
        Some((state, work))
    }
    fn instantiate(
        &self,
        data: &Arc<BoundRouter>,
        chunk: ChunkPos,
        budget: NoiseChunkWorkBudget,
    ) -> Option<NoiseChunkSession> {
        if !Arc::ptr_eq(data, &self.data) {
            return None;
        }
        let (state, dynamic) = self.rebuild(chunk)?;
        let total = self.static_work.checked_add(dynamic.used())?;
        let mut work = SampleWork::new(budget.0 as usize);
        // Private staging does not touch the caller. An insufficient allowance
        // uses the original constructor, preserving its exact first refusal.
        if !work.try_charge_public_group(total) {
            return None;
        }
        Some(NoiseChunkSession {
            data: Arc::clone(data),
            chunk,
            view: clone_view(&self.view)?,
            state,
            fields: self.fields,
            material: self.material,
            climate: self.climate,
            work,
            surface: SurfaceCache::new(),
            biome_phase: Lifecycle::Fresh,
        })
    }
}
impl SeededRouter {
    pub(crate) fn compile_natural_noise_plan(&self) -> Option<NaturalNoisePlan> {
        NaturalNoisePlan::compile(Arc::clone(&self.data))
    }
    pub(crate) fn fresh_natural_noise_chunk(
        &self,
        plan: Option<&NaturalNoisePlan>,
        chunk: ChunkPos,
        budget: NoiseChunkWorkBudget,
    ) -> Result<NoiseChunkSession, NoiseChunkError> {
        if let Some(session) = plan.and_then(|p| p.instantiate(&self.data, chunk, budget)) {
            return Ok(session);
        }
        self.fresh_paper_noise_chunk(chunk, budget)
    }
}
fn topological(view: &BoundView) -> bool {
    view.nodes
        .iter()
        .enumerate()
        .all(|(index, node)| match &node.kind {
            BoundKind::Alias(child) | BoundKind::MulOrAdd { input: child, .. } => *child < index,
            BoundKind::Original(kind) => {
                (0..kind.children()).all(|i| kind.child(i).is_some_and(|child| child < index))
            }
        })
}
fn node_allocations(node: &BoundNode) -> usize {
    let points = match &node.kind {
        BoundKind::Original(NodeKind::SplineMultipoint { points, .. }) => {
            points.len() * size_of::<Point>()
        }
        _ => 0,
    };
    points
        + node
            .blended
            .as_ref()
            .map_or(0, |v| v.retained_bytes() + 2 * size_of::<usize>())
        + node.spline.as_ref().map_or(0, |v| {
            v.spline.retained_bytes() + v.coordinates.len() * size_of::<usize>()
        })
}
fn clone_slice<T: Clone>(input: &[T]) -> Option<Box<[T]>> {
    let mut values = Vec::new();
    values.try_reserve_exact(input.len()).ok()?;
    values.extend_from_slice(input);
    Some(values.into_boxed_slice())
}
fn clone_view(view: &BoundView) -> Option<BoundView> {
    let mut nodes = Vec::new();
    nodes.try_reserve_exact(view.nodes.len()).ok()?;
    for node in &view.nodes {
        let kind = match &node.kind {
            BoundKind::Original(NodeKind::SplineMultipoint { coordinate, points }) => {
                BoundKind::Original(NodeKind::SplineMultipoint {
                    coordinate: *coordinate,
                    points: clone_slice(points)?,
                })
            }
            kind => kind.clone(),
        };
        let spline = match &node.spline {
            Some(spline) => Some(BoundSpline {
                spline: spline.spline.clone(),
                coordinates: clone_slice(&spline.coordinates)?,
            }),
            None => None,
        };
        nodes.push(BoundNode {
            origin: node.origin,
            resource: node.resource,
            kind,
            bounds: node.bounds,
            noise: node.noise,
            blended: node.blended.clone(),
            spline,
        });
    }
    Some(BoundView {
        pass: view.pass,
        nodes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        density::{ClimateQuartPos, RouterTemplate},
        WorldSeed,
    };
    use ferrumc_registry::worldgen::NoiseSettingsResourceId;
    fn router(seed: i64) -> SeededRouter {
        RouterTemplate::compile_builtin(NoiseSettingsResourceId::from_name("overworld").unwrap())
            .unwrap()
            .bind_seed(WorldSeed::new(seed))
            .unwrap()
    }
    fn full() -> NoiseChunkWorkBudget {
        NoiseChunkWorkBudget::new(MAX_NOISE_CHUNK_WORK).unwrap()
    }
    #[test]
    fn constructor_plan_native_census_and_exact_origin() {
        let router = router(0);
        let plan = router
            .compile_natural_noise_plan()
            .expect("sealed builtin plan must fit and verify");
        println!("CONSTRUCTOR_PLAN_NATIVE retained={} cap={PLAN_BYTES} conservative_compile={COMPILE_CENSUS} compile_cap={COMPILE_BYTES} nodes={} recipes={} static_work={}; excludes original shared router and allocator overhead",plan.native_bytes(),plan.view.nodes.len(),plan.recipes.len(),plan.static_work);
        assert!(plan.native_bytes() <= PLAN_BYTES);
        const {
            assert!(COMPILE_CENSUS <= COMPILE_BYTES);
        }
        let original = router
            .fresh_paper_noise_chunk(ChunkPos::ORIGIN, full())
            .unwrap();
        let replay = plan
            .instantiate(&router.data, ChunkPos::ORIGIN, full())
            .unwrap();
        assert_eq!(original.work_used(), replay.work_used());
        assert_eq!(original.carver_history(), replay.carver_history());
    }
    #[test]
    fn constructor_plan_exact_position_signed_seed_state_work_and_climate() {
        for seed in [0, 17, i64::MIN, i64::MAX] {
            let router = router(seed);
            let plan = router.compile_natural_noise_plan().unwrap();
            for pos in [
                ChunkPos::ORIGIN,
                ChunkPos::new(-1, -1),
                ChunkPos::new(100, 100),
                ChunkPos::new(-1_875_000, -1_875_000),
                ChunkPos::new(1_874_999, 1_874_999),
            ] {
                let mut original = router.fresh_paper_noise_chunk(pos, full()).unwrap();
                let mut replay = plan.instantiate(&router.data, pos, full()).unwrap();
                assert!(
                    original.state.plan_equivalent(&replay.state),
                    "seed={seed} pos={pos:?}"
                );
                assert_eq!(
                    original.work_used(),
                    replay.work_used(),
                    "seed={seed} pos={pos:?}"
                );
                for y in [-16, 0, 79] {
                    let quart = ClimateQuartPos::new(pos.x() * 4, y, pos.z() * 4).unwrap();
                    let a = original.sample_climate(quart).unwrap();
                    let b = replay.sample_climate(quart).unwrap();
                    assert_eq!(a.values().map(f32::to_bits), b.values().map(f32::to_bits));
                    assert_eq!(a.quantized(), b.quantized());
                    assert_eq!(original.work_used(), replay.work_used());
                    assert!(original.state.plan_equivalent(&replay.state));
                }
            }
        }
    }
    #[test]
    fn constructor_plan_full_and_short_allowances_preserve_first_refusal() {
        let router = router(-17);
        let plan = router.compile_natural_noise_plan().unwrap();
        for pos in [
            ChunkPos::ORIGIN,
            ChunkPos::new(-1, -1),
            ChunkPos::new(100, 100),
        ] {
            let n = router
                .fresh_paper_noise_chunk(pos, full())
                .unwrap()
                .work_used();
            for allowance in [1, 49, 50, 100, 5000, n - 1, n, n + 1] {
                let budget = NoiseChunkWorkBudget::new(u32::try_from(allowance).unwrap()).unwrap();
                let original = router.fresh_paper_noise_chunk(pos, budget);
                let replay = router.fresh_natural_noise_chunk(Some(&plan), pos, budget);
                match (original, replay) {
                    (Err(a), Err(b)) => assert_eq!(a, b),
                    (Ok(mut a), Ok(mut b)) => {
                        assert_eq!(a.work_used(), b.work_used());
                        assert_eq!(a.carver_history(), b.carver_history());
                        let quart = ClimateQuartPos::new(pos.x() * 4, -16, pos.z() * 4).unwrap();
                        assert_eq!(a.sample_climate(quart).err(), b.sample_climate(quart).err());
                        assert_eq!(a.work_used(), b.work_used());
                        assert_eq!(a.carver_history(), b.carver_history());
                    }
                    _ => panic!("constructor outcome changed for {pos:?} allowance{allowance}"),
                }
            }
        }
    }
    #[test]
    fn constructor_plan_foreign_router_and_invalid_domain_fall_back() {
        let owner = router(0);
        let same_seed = router(0);
        let other_seed = router(17);
        let plan = owner.compile_natural_noise_plan().unwrap();
        for foreign in [&same_seed, &other_seed] {
            assert!(plan
                .instantiate(&foreign.data, ChunkPos::ORIGIN, full())
                .is_none());
            let a = foreign
                .fresh_paper_noise_chunk(ChunkPos::ORIGIN, full())
                .unwrap();
            let b = foreign
                .fresh_natural_noise_chunk(Some(&plan), ChunkPos::ORIGIN, full())
                .unwrap();
            assert_eq!(a.work_used(), b.work_used());
            assert_eq!(a.carver_history(), b.carver_history());
        }
        for pos in [
            ChunkPos::new(i32::MIN, 0),
            ChunkPos::new(0, i32::MAX),
            ChunkPos::new(1_875_000, 0),
        ] {
            assert!(plan.instantiate(&owner.data, pos, full()).is_none());
            assert_eq!(
                owner.fresh_paper_noise_chunk(pos, full()).err(),
                owner
                    .fresh_natural_noise_chunk(Some(&plan), pos, full())
                    .err()
            );
        }
    }
}
