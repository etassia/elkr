//! The Force algorithm tests: simulation behavior, determinism, options,
//! edge routing, and the error surface.

use elkr::elk::{ElkEdge, ElkGraph, ElkNode};
use elkr::layout::{self, LayoutError};

fn node(id: &str, w: f64, h: f64) -> ElkNode {
    ElkNode {
        id: id.to_string(),
        x: None,
        y: None,
        width: Some(w),
        height: Some(h),
        children: Vec::new(),
        labels: Vec::new(),
        ports: Vec::new(),
    }
}

fn edge(id: &str, src: &str, tgt: &str) -> ElkEdge {
    ElkEdge {
        id: id.to_string(),
        sources: vec![src.to_string()],
        targets: vec![tgt.to_string()],
        sections: Vec::new(),
    }
}

fn serde_json_to<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_string(v).expect("serialize")
}

fn force_options() -> Vec<(&'static str, String)> {
    vec![
        ("elk.algorithm", "force".to_string()),
        // the default temperature of 0.001 caps displacement at a
        // fraction of a pixel; the tests raise it so the simulation
        // actually converges
        ("elk.force.temperature", "60".to_string()),
        ("elk.force.iterations", "400".to_string()),
    ]
}

fn graph_with_options(
    nodes: Vec<ElkNode>,
    edges: Vec<ElkEdge>,
    options: &[(&str, String)],
) -> ElkGraph {
    let mut g = ElkGraph {
        id: "root".to_string(),
        children: nodes,
        edges,
        layout_options: Default::default(),
    };
    for (k, v) in options {
        g.layout_options.insert(k, v);
    }
    g
}

fn connected_graph() -> ElkGraph {
    graph_with_options(
        vec![
            node("a", 40.0, 40.0),
            node("b", 40.0, 40.0),
            node("c", 40.0, 40.0),
            node("d", 40.0, 40.0),
            node("e", 40.0, 40.0),
        ],
        vec![
            edge("ab", "a", "b"),
            edge("bc", "b", "c"),
            edge("cd", "c", "d"),
        ],
        &force_options(),
    )
}

fn centers(g: &ElkGraph) -> std::collections::BTreeMap<String, (f64, f64)> {
    g.children
        .iter()
        .map(|n| {
            (
                n.id.clone(),
                (
                    n.x.unwrap() + n.width.unwrap() / 2.0,
                    n.y.unwrap() + n.height.unwrap() / 2.0,
                ),
            )
        })
        .collect()
}

fn distance(g: &ElkGraph, a: &str, b: &str) -> f64 {
    let c = centers(g);
    let (ax, ay) = c[a];
    let (bx, by) = c[b];
    ((ax - bx).powi(2) + (ay - by).powi(2)).sqrt()
}

#[test]
fn lays_out_every_node() {
    let mut g = connected_graph();
    layout::by_name("force").unwrap().layout(&mut g).unwrap();
    assert_eq!(g.children.len(), 5);
    for n in &g.children {
        assert!(n.x.unwrap() >= 12.0, "node {} left of padding", n.id);
        assert!(n.y.unwrap() >= 12.0, "node {} above padding", n.id);
    }
}

#[test]
fn connected_chain_collapses_below_the_spring_equilibrium() {
    // the spring attraction (distance/10) meets the repulsion (5/d^2) at
    // d ~= 3.68: with a hot temperature the chain must converge well
    // inside a node's own extent
    let mut g = connected_graph();
    layout::by_name("force").unwrap().layout(&mut g).unwrap();
    for (a, b) in [("a", "b"), ("b", "c"), ("c", "d")] {
        let d = distance(&g, a, b);
        assert!(d < 40.0, "connected {a}-{b} stayed {d} apart");
    }
}

#[test]
fn connected_nodes_end_closer_than_unconnected() {
    // a-b-c-d form a path; e is unconnected. Each connected pair must
    // end closer than the e-to-path distance on average: the springs
    // pull endpoints together while e only feels repulsion.
    let mut g = connected_graph();
    layout::by_name("force").unwrap().layout(&mut g).unwrap();
    let edge_mean =
        (distance(&g, "a", "b") + distance(&g, "b", "c") + distance(&g, "c", "d")) / 3.0;
    let far_mean = (distance(&g, "e", "a") + distance(&g, "e", "b") + distance(&g, "e", "d")) / 3.0;
    assert!(
        edge_mean < far_mean,
        "connected mean {edge_mean} should be below unconnected mean {far_mean}"
    );
}

#[test]
fn same_input_same_output() {
    let run = || {
        let mut g = connected_graph();
        layout::by_name("force").unwrap().layout(&mut g).unwrap();
        serde_json_to(&g)
    };
    assert_eq!(run(), run(), "the force layout must be byte-identical");
}

#[test]
fn preset_positions_are_preserved_as_the_start() {
    // with zero iterations the simulation never runs, so fully preset
    // nodes keep their start, shifted by the padding toward (12, 12)
    let mut g = graph_with_options(
        vec![node("a", 40.0, 40.0), node("b", 40.0, 40.0)],
        vec![edge("ab", "a", "b")],
        &[
            ("elk.algorithm", "force".to_string()),
            ("elk.force.iterations", "0".to_string()),
        ],
    );
    g.children[0].x = Some(100.0);
    g.children[0].y = Some(200.0);
    g.children[1].x = Some(300.0);
    g.children[1].y = Some(150.0);
    layout::by_name("force").unwrap().layout(&mut g).unwrap();
    assert_eq!(g.children[0].x, Some(12.0));
    assert_eq!(g.children[0].y, Some(62.0));
    assert_eq!(g.children[1].x, Some(212.0));
    assert_eq!(g.children[1].y, Some(12.0));
}

#[test]
fn edges_route_center_to_center() {
    let mut g = graph_with_options(
        vec![node("a", 40.0, 40.0), node("b", 40.0, 40.0)],
        vec![edge("ab", "a", "b")],
        &[
            ("elk.algorithm", "force".to_string()),
            ("elk.force.iterations", "0".to_string()),
        ],
    );
    g.children[0].x = Some(0.0);
    g.children[0].y = Some(0.0);
    g.children[1].x = Some(100.0);
    g.children[1].y = Some(0.0);
    layout::by_name("force").unwrap().layout(&mut g).unwrap();
    let s = &g.edges[0].sections[0];
    // both nodes shift by the padding: a to (12, 12), b to (112, 12)
    assert_eq!((s.start_point.x, s.start_point.y), (32.0, 32.0));
    assert_eq!((s.end_point.x, s.end_point.y), (132.0, 32.0));
    assert_eq!(s.incoming_shape.as_deref(), Some("a"));
    assert_eq!(s.outgoing_shape.as_deref(), Some("b"));
}

#[test]
fn bare_and_elk_prefixed_options_agree() {
    let build = |temp_key: &str| {
        graph_with_options(
            vec![
                node("a", 40.0, 40.0),
                node("b", 40.0, 40.0),
                node("c", 40.0, 40.0),
            ],
            vec![edge("ab", "a", "b")],
            &[
                ("elk.algorithm", "force".to_string()),
                (temp_key, "30".to_string()),
            ],
        )
    };
    let mut bare = build("temperature");
    let mut elk = build("elk.force.temperature");
    layout::by_name("force").unwrap().layout(&mut bare).unwrap();
    layout::by_name("force").unwrap().layout(&mut elk).unwrap();
    // the options map rides the serialized output, so compare geometry
    assert_eq!(serde_json_to(&bare.children), serde_json_to(&elk.children));
}

#[test]
fn self_loop_is_ignored() {
    let mut g = graph_with_options(
        vec![node("a", 40.0, 40.0)],
        vec![edge("loop", "a", "a")],
        &force_options(),
    );
    layout::by_name("force").unwrap().layout(&mut g).unwrap();
    assert!(g.edges[0].sections.is_empty());
}

#[test]
fn unknown_edge_endpoint_is_an_error() {
    let mut g = graph_with_options(
        vec![node("a", 40.0, 40.0)],
        vec![edge("bad", "a", "ghost")],
        &force_options(),
    );
    let err = layout::by_name("force")
        .unwrap()
        .layout(&mut g)
        .unwrap_err();
    assert!(matches!(err, LayoutError::UnknownNode { .. }), "{err:?}");
}

#[test]
fn empty_graph_is_a_noop() {
    let mut g = graph_with_options(vec![], vec![], &force_options());
    layout::by_name("force").unwrap().layout(&mut g).unwrap();
    assert!(g.children.is_empty());
}

#[test]
fn apply_dispatches_on_elk_algorithm() {
    let mut g = connected_graph();
    layout::apply(&mut g).unwrap();
    assert!(g.children[0].x.is_some());
}

#[test]
fn elkrb_scatter_spreads_inside_the_area_square() {
    // with zero iterations the scatter is the output: nodes must land in
    // a square of side sqrt(sum of padded areas), padded
    let mut g = graph_with_options(
        vec![
            node("a", 40.0, 40.0),
            node("b", 40.0, 40.0),
            node("c", 40.0, 40.0),
        ],
        vec![],
        &[
            ("elk.algorithm", "force".to_string()),
            ("elk.force.iterations", "0".to_string()),
        ],
    );
    layout::by_name("force").unwrap().layout(&mut g).unwrap();
    let side = (3.0 * 60.0 * 60.0f64).sqrt();
    for n in &g.children {
        let x = n.x.unwrap();
        let y = n.y.unwrap();
        assert!(x >= 12.0 && x <= 12.0 + side, "x {x} outside scatter");
        assert!(y >= 12.0 && y <= 12.0 + side, "y {y} outside scatter");
    }
}
