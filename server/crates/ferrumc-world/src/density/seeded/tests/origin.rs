use super::*;

#[test]
fn checked_sparse_registered_origins_and_both_visitor_views_keep_their_caps() {
    use crate::density::bind::{PreparedGraph, SourceConstruction, VisitorPass};
    for resource in ferrumc_registry::worldgen::noise_settings_resources() {
        let template = RouterTemplate::compile_builtin(resource).unwrap();
        let prepared = PreparedGraph::new(&template.graph).unwrap();
        let expected = match resource.resource_location() {
            "minecraft:overworld" | "minecraft:amplified" | "minecraft:large_biomes" => {
                (2560, 3032, 71)
            }
            "minecraft:end" => (2457, 2907, 35),
            _ => (2433, 2887, 35),
        };
        assert_eq!(
            (
                prepared.graph.stats.vertices(),
                prepared.graph.stats.edges(),
                prepared.graph.stats.depth()
            ),
            expected
        );
        assert_eq!(
            prepared
                .origins
                .iter()
                .filter(|origin| origin.construction == SourceConstruction::Registered)
                .count(),
            162
        );
        assert_eq!(prepared.registered_roots.len(), 35);
        let router = template.bind_seed(WorldSeed::new(-1)).unwrap();
        for (view, pass) in [
            (&router.data.router, VisitorPass::SeedRouter),
            (&router.data.climate, VisitorPass::Climate),
        ] {
            assert_eq!(view.pass, pass);
            assert_eq!(view.nodes.len(), expected.0);
            for (index, node) in view.nodes.iter().enumerate() {
                assert_eq!(node.origin, prepared.origins[index]);
                let source = &template.graph.nodes[node.origin.index];
                assert_eq!(node.resource, source.resource);
                let holder = match source.kind {
                    NodeKind::Holder { origin, .. } | NodeKind::Coordinate { origin, .. } => {
                        Some(origin)
                    }
                    _ => None,
                };
                assert_eq!(node.origin.holder, holder);
            }
        }
        let shift = DensityFunctionResourceId::from_name("shift_x").unwrap();
        assert_ne!(
            router.data.definitions[&shift],
            router.data.registered_definitions[&shift]
        );
        let direct_root = router.data.definitions[&shift];
        let registered_root = router.data.registered_definitions[&shift];
        let BoundKind::Original(NodeKind::Marker { input: direct, .. }) =
            router.data.router.nodes[direct_root].kind
        else {
            panic!("decoded marker")
        };
        let BoundKind::Original(NodeKind::Marker {
            input: registered, ..
        }) = router.data.router.nodes[registered_root].kind
        else {
            panic!("native marker")
        };
        assert!(matches!(
            router.data.router.nodes[direct].kind,
            BoundKind::Original(NodeKind::Holder { .. })
        ));
        assert!(matches!(
            router.data.router.nodes[registered].kind,
            BoundKind::Original(NodeKind::Marker { .. })
        ));
        let pos = BlockPos::new(13, 73, -29);
        let mut direct_work = SampleWork::new(MAX_DENSITY_SAMPLE_WORK);
        let mut native_work = SampleWork::new(MAX_DENSITY_SAMPLE_WORK);
        let a = evaluate::sample(
            &router.data,
            &router.data.router,
            direct_root,
            pos,
            &mut direct_work,
            "probe",
        )
        .unwrap();
        let b = evaluate::sample(
            &router.data,
            &router.data.router,
            registered_root,
            pos,
            &mut native_work,
            "probe",
        )
        .unwrap();
        assert_eq!(a.to_bits(), b.to_bits());
        assert_eq!(direct_work.used(), native_work.used() + 2);
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "Keeps the complete original reference or shared-meter acceptance matrix together."
)]
fn origin_preparation_checks_pre_growth_vertex_edge_and_graph_shape_limits() {
    use crate::density::bind::PreparedGraph;
    use crate::density::graph::Node;
    let template =
        RouterTemplate::compile_builtin(NoiseSettingsResourceId::from_name("overworld").unwrap())
            .unwrap();
    let graph = |nodes| crate::density::graph::Graph {
        nodes,
        roots: template.graph.roots.clone(),
        router_roots: template.graph.router_roots,
        stats: template.graph.stats,
        source_bytes: template.graph.source_bytes,
        noises: template.graph.noises.clone(),
    };
    let nodes = (0..4097)
        .map(|_| Node {
            resource: "limit",
            kind: NodeKind::Constant(0.0),
            depth: 1,
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    assert!(matches!(
        PreparedGraph::new(&graph(nodes)).unwrap_err().kind(),
        DensityBindErrorKind::Limit {
            count: 4097,
            maximum: 4096
        }
    ));
    let mut nodes = template
        .graph
        .nodes
        .iter()
        .map(|n| Node {
            resource: n.resource,
            kind: n.kind.clone(),
            depth: n.depth,
        })
        .collect::<Vec<_>>();
    nodes.resize_with(4096, || Node {
        resource: "fill",
        kind: NodeKind::Constant(0.0),
        depth: 1,
    });
    assert!(matches!(
        PreparedGraph::new(&graph(nodes.into_boxed_slice()))
            .unwrap_err()
            .kind(),
        DensityBindErrorKind::Limit {
            count: 4097,
            maximum: 4096
        }
    ));
    let mut nodes = (0..4096)
        .map(|index| Node {
            resource: "edges",
            kind: if index == 0 {
                NodeKind::Constant(0.0)
            } else {
                NodeKind::Binary {
                    operation: Binary::Add,
                    left: 0,
                    right: 0,
                }
            },
            depth: if index == 0 { 1 } else { 2 },
        })
        .collect::<Vec<_>>();
    for node in &mut nodes[1..=2] {
        node.kind = NodeKind::RangeChoice {
            input: 0,
            min: 0.0,
            max: 1.0,
            inside: 0,
            outside: 0,
        };
    }
    let mut source = graph(nodes.into_boxed_slice());
    assert_eq!(
        PreparedGraph::new(&source).unwrap().graph.stats.edges(),
        8192
    );
    source.nodes[3].kind = NodeKind::RangeChoice {
        input: 0,
        min: 0.0,
        max: 1.0,
        inside: 0,
        outside: 0,
    };
    assert!(matches!(
        PreparedGraph::new(&source).unwrap_err().kind(),
        DensityBindErrorKind::Limit {
            count: 8193,
            maximum: 8192
        }
    ));
    source.nodes[3].kind = NodeKind::Binary {
        operation: Binary::Add,
        left: 0,
        right: 0,
    };
    source.nodes[0].kind = NodeKind::Holder {
        origin: HolderOrigin::Direct,
        target: 0,
    };
    assert!(PreparedGraph::new(&source).is_err());
}
