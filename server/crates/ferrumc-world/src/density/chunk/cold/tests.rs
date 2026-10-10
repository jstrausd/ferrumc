use super::*;
use crate::density::chunk::tests::{budget, router};
use ferrumc_math::ChunkPos;

pub(super) mod oracle;

fn constant(data: &BoundRouter, value: f64) -> BoundNode {
    let mut node = data.router.nodes[0].clone();
    node.kind = BoundKind::Original(NodeKind::Constant(value));
    node.bounds = bind::pair(value, value);
    node.noise = None;
    node.blended = None;
    node.spline = None;
    node
}
fn is_limit(e: &NoiseChunkError, kind: NoiseChunkLimit) {
    assert!(
        matches!(e.kind(),NoiseChunkErrorKind::Limit {limit,..} if *limit==kind),
        "{e:?}"
    );
}

#[test]
fn equality_and_identity_arenas_admit_full_capacity_then_refuse_append() {
    let r = router("minecraft:nether", 0);
    let mut work = SampleWork::new(budget().units() as usize);
    let state = ChunkState::new(
        &r.data,
        ChunkPos::ORIGIN,
        r.data.metadata.geometry,
        &mut work,
    )
    .unwrap();
    let mut c = Cold::new(&r.data, state, &mut work).unwrap();
    for i in 0..4096 {
        assert_eq!(
            c.intern(constant(&r.data, i as f64), 0, None, None)
                .unwrap(),
            i
        );
    }
    is_limit(
        &c.intern(constant(&r.data, 4096.0), 0, None, None)
            .unwrap_err(),
        NoiseChunkLimit::EqualityKeys,
    );
    assert_eq!(c.view.nodes.len(), 4096);
    for i in 0..4096 {
        assert_eq!(
            c.keys.identity(Identity::FreshNoise, c.work).unwrap(),
            i + 1
        );
    }
    is_limit(
        &c.keys.identity(Identity::FreshNoise, c.work).unwrap_err(),
        NoiseChunkLimit::IdentityTokens,
    );
}

#[test]
fn graph_vertex_edge_metadata_and_visit_limits_precede_growth() {
    let r = router("minecraft:nether", 0);
    let mut work = SampleWork::new(budget().units() as usize);
    let state = ChunkState::new(
        &r.data,
        ChunkPos::ORIGIN,
        r.data.metadata.geometry,
        &mut work,
    )
    .unwrap();
    let mut c = Cold::new(&r.data, state, &mut work).unwrap();
    let node = constant(&r.data, 0.0);
    c.intern(node.clone(), 0, None, None).unwrap();
    c.view.nodes.resize(4096, node.clone());
    is_limit(
        &c.intern(constant(&r.data, 1.0), 0, None, None).unwrap_err(),
        NoiseChunkLimit::Vertices,
    );
    c.view.nodes.truncate(1);
    let mut edge = node.clone();
    edge.kind = BoundKind::Original(NodeKind::Binary {
        operation: Binary::Add,
        left: 0,
        right: 0,
    });
    c.edges = 8190;
    c.intern(edge, 0, None, None).unwrap();
    assert_eq!(c.edges, 8192);
    let mut edge = node.clone();
    edge.kind = BoundKind::Original(NodeKind::Unary {
        operation: crate::density::graph::Unary::Square,
        input: 0,
    });
    is_limit(
        &c.intern(edge, 0, None, None).unwrap_err(),
        NoiseChunkLimit::Edges,
    );
    c.edges = 0;
    c.graph_bytes = 2 * 1024 * 1024;
    let mut points = node;
    points.kind = BoundKind::Original(NodeKind::SplineMultipoint {
        coordinate: 0,
        points: vec![crate::density::graph::Point {
            location: 0.0,
            derivative: 0.0,
            value: 0,
        }]
        .into_boxed_slice(),
    });
    is_limit(
        &c.intern(points, 0, Some((1, 2)), None).unwrap_err(),
        NoiseChunkLimit::GraphBytes,
    );
    c.visit(96).unwrap();
    let used = c.work.used();
    let visits = c.visits;
    is_limit(&c.visit(97).unwrap_err(), NoiseChunkLimit::Depth);
    assert_eq!(c.work.used(), used);
    assert_eq!(c.visits, visits);
    c.visits = 131_071;
    c.visit(1).unwrap();
    assert_eq!(c.visits, 131_072);
    let used = c.work.used();
    is_limit(&c.visit(1).unwrap_err(), NoiseChunkLimit::ColdVisits);
    assert_eq!(c.work.used(), used);
}

#[test]
fn visitor_map_and_wrapper_capacity_preserve_first_registration() {
    let r = router("minecraft:nether", 0);
    let mut work = SampleWork::new(budget().units() as usize);
    let state = ChunkState::new(
        &r.data,
        ChunkPos::ORIGIN,
        r.data.metadata.geometry,
        &mut work,
    )
    .unwrap();
    let mut c = Cold::new(&r.data, state, &mut work).unwrap();
    let zero = c.intern(constant(&r.data, 0.0), 0, None, None).unwrap();
    let one = c.intern(constant(&r.data, 1.0), 0, None, None).unwrap();
    c.wrap_entries = 4095;
    assert_eq!(c.wrap(zero, 1).unwrap(), zero);
    assert_eq!(c.wrap_entries, 4096);
    // Existing visitor-map hits remain valid at capacity.
    assert_eq!(c.wrap(zero, 1).unwrap(), zero);
    is_limit(
        &c.wrap(one, 1).unwrap_err(),
        NoiseChunkLimit::VisitorEntries,
    );
    for _ in 2..128 {
        c.state.add_wrapper(Marker::FlatCache, zero).unwrap();
    }
    assert_eq!(c.state.wrappers.len(), 128);
    let cachebytes = c.state.cache_bytes;
    is_limit(
        &c.state
            .add_wrapper(Marker::CacheAllInCell, zero)
            .unwrap_err(),
        NoiseChunkLimit::Wrappers,
    );
    assert_eq!(c.state.wrappers.len(), 128);
    assert_eq!(c.state.cache_bytes, cachebytes);
}

#[test]
fn joint_validator_rejects_wrong_density_spline_coordinate_shapes_and_cycles() {
    let r = router("minecraft:nether", 0);
    for kind in [
        NodeKind::Spline(0),
        NodeKind::SplineMultipoint {
            coordinate: 0,
            points: vec![crate::density::graph::Point {
                location: 0.0,
                derivative: 0.0,
                value: 0,
            }]
            .into_boxed_slice(),
        },
        NodeKind::Unary {
            operation: crate::density::graph::Unary::Square,
            input: 1,
        },
    ] {
        let mut work = SampleWork::new(budget().units() as usize);
        let state = ChunkState::new(
            &r.data,
            ChunkPos::ORIGIN,
            r.data.metadata.geometry,
            &mut work,
        )
        .unwrap();
        let mut c = Cold::new(&r.data, state, &mut work).unwrap();
        let node = constant(&r.data, 0.0);
        c.view.nodes.push(node.clone());
        let mut parent = node;
        parent.kind = BoundKind::Original(kind);
        c.view.nodes.push(parent);
        assert!(matches!(
            c.validate(&[1; 15], 1, &[1; 6]).unwrap_err().kind(),
            NoiseChunkErrorKind::InvalidGraph
        ));
    }
}

#[test]
fn large_admitted_geometry_refuses_scratch_before_allocating_pool() {
    let mut r = router("minecraft:overworld", 0);
    let data = std::sync::Arc::get_mut(&mut r.data).unwrap();
    data.metadata.geometry = crate::density::NoiseGeometry {
        min_y: 0,
        height: 512,
        horizontal: 4,
        vertical: 128,
    };
    let error = r
        .fresh_paper_noise_chunk(ChunkPos::ORIGIN, budget())
        .err()
        .unwrap();
    is_limit(&error, NoiseChunkLimit::ScratchBytes);
}
