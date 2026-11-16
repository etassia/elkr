//! The structural algorithms' tests: Stress and MRTree.

use elkr::elk::{ElkEdge, ElkGraph, ElkNode};
use elkr::layout;

fn serde_json_to<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_string(v).expect("serialize")
}

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
        labels: Vec::new(),
        layout_options: Default::default(),
    }
}

fn graph_with(nodes: Vec<ElkNode>, edges: Vec<ElkEdge>, algorithm: &str) -> ElkGraph {
    let mut g = ElkGraph {
        id: "root".to_string(),
        children: nodes,
        edges,
        layout_options: Default::default(),
    };
    g.layout_options.insert("elk.algorithm", algorithm);
    g
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

// ---- Stress ----

#[test]
fn stress_pulls_connected_nodes_closer_than_unconnected() {
    let mut g = graph_with(
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
        "stress",
    );
    layout::by_name("stress").unwrap().layout(&mut g).unwrap();
    let edge_mean =
        (distance(&g, "a", "b") + distance(&g, "b", "c") + distance(&g, "c", "d")) / 3.0;
    let far_mean = (distance(&g, "e", "a") + distance(&g, "e", "b") + distance(&g, "e", "d")) / 3.0;
    assert!(
        edge_mean < far_mean,
        "connected mean {edge_mean} should be below unconnected mean {far_mean}"
    );
}

#[test]
fn stress_converges_the_chain_below_two_hops() {
    // the ideal distance matrix is hop counts: b and d sit two hops
    // apart on the path, so their ideal is 2.0 and the solution lands
    // near it
    let mut g = graph_with(
        vec![
            node("a", 40.0, 40.0),
            node("b", 40.0, 40.0),
            node("c", 40.0, 40.0),
            node("d", 40.0, 40.0),
        ],
        vec![
            edge("ab", "a", "b"),
            edge("bc", "b", "c"),
            edge("cd", "c", "d"),
        ],
        "stress",
    );
    layout::by_name("stress").unwrap().layout(&mut g).unwrap();
    let bd = distance(&g, "b", "d");
    assert!(
        (bd - 2.0).abs() < 1.0,
        "the two-hop pair settled {bd} from its ideal 2.0"
    );
}

#[test]
fn stress_is_deterministic() {
    let build = || {
        graph_with(
            vec![
                node("a", 40.0, 40.0),
                node("b", 40.0, 40.0),
                node("c", 40.0, 40.0),
            ],
            vec![edge("ab", "a", "b")],
            "stress",
        )
    };
    let run = |mut g: ElkGraph| {
        layout::by_name("stress").unwrap().layout(&mut g).unwrap();
        serde_json_to(&g)
    };
    assert_eq!(run(build()), run(build()));
}

#[test]
fn stress_skips_edges_with_unknown_endpoints() {
    let mut g = graph_with(
        vec![node("a", 40.0, 40.0), node("b", 40.0, 40.0)],
        vec![edge("bad", "a", "ghost")],
        "stress",
    );
    layout::by_name("stress").unwrap().layout(&mut g).unwrap();
    assert!(g.edges[0].sections.is_empty());
}

#[test]
fn stress_empty_graph_is_a_noop() {
    let mut g = graph_with(vec![], vec![], "stress");
    layout::by_name("stress").unwrap().layout(&mut g).unwrap();
    assert!(g.children.is_empty());
}

// ---- MRTree ----

#[test]
fn mrtree_centers_a_parent_over_its_children() {
    // a -> b, a -> c: b and c sit one level down, a centers between them
    let mut g = graph_with(
        vec![
            node("a", 40.0, 40.0),
            node("b", 40.0, 40.0),
            node("c", 40.0, 40.0),
        ],
        vec![edge("ab", "a", "b"), edge("ac", "a", "c")],
        "mrtree",
    );
    layout::by_name("mrtree").unwrap().layout(&mut g).unwrap();
    let b = centers(&g)["b"];
    let c = centers(&g)["c"];
    let a = centers(&g)["a"];
    assert!(
        a.0 > b.0.min(c.0) && a.0 < b.0.max(c.0),
        "the parent is not between its children: {b:?} {a:?} {c:?}"
    );
    assert_eq!(a.1, b.1 - 80.0);
    assert_eq!(b.1, c.1);
}

#[test]
fn mrtree_side_by_side_trees_advance_by_width() {
    // two roots with no edge between them: the second tree starts
    // after the first tree's width plus the spacing
    let mut g = graph_with(
        vec![
            node("r1", 40.0, 40.0),
            node("r2", 40.0, 40.0),
            node("l1", 40.0, 40.0),
            node("l2", 40.0, 40.0),
        ],
        vec![edge("t1", "r1", "l1"), edge("t2", "r2", "l2")],
        "mrtree",
    );
    layout::by_name("mrtree").unwrap().layout(&mut g).unwrap();
    let (r1, r2, l1, l2) = (
        at_x(&g, "r1"),
        at_x(&g, "r2"),
        at_x(&g, "l1"),
        at_x(&g, "l2"),
    );
    // root one sits above leaf one; root two's tree starts to the
    // right of leaf one
    assert_eq!(r1, l1);
    assert!(r2 > l1 + 40.0, "the trees overlap");
    assert_eq!(r2, l2);
}

#[test]
fn mrtree_survives_a_cycle() {
    // a -> b -> a: the visited guard ends the recursion
    let mut g = graph_with(
        vec![node("a", 40.0, 40.0), node("b", 40.0, 40.0)],
        vec![edge("ab", "a", "b"), edge("ba", "b", "a")],
        "mrtree",
    );
    layout::by_name("mrtree").unwrap().layout(&mut g).unwrap();
    assert!(g.children[0].x.is_some());
}

#[test]
fn mrtree_every_node_is_a_root_when_all_have_incoming() {
    let mut g = graph_with(
        vec![node("a", 40.0, 40.0), node("b", 40.0, 40.0)],
        vec![edge("ab", "a", "b"), edge("ba", "b", "a")],
        "mrtree",
    );
    layout::by_name("mrtree").unwrap().layout(&mut g).unwrap();
    // both trees laid out side by side, neither at the same x
    assert_ne!(at_x(&g, "a"), at_x(&g, "b"));
}

#[test]
fn mrtree_is_deterministic() {
    let build = || {
        graph_with(
            vec![
                node("a", 40.0, 40.0),
                node("b", 40.0, 40.0),
                node("c", 40.0, 40.0),
                node("d", 40.0, 40.0),
            ],
            vec![
                edge("ab", "a", "b"),
                edge("ac", "a", "c"),
                edge("bd", "b", "d"),
            ],
            "mrtree",
        )
    };
    let run = |mut g: ElkGraph| {
        layout::by_name("mrtree").unwrap().layout(&mut g).unwrap();
        serde_json_to(&g)
    };
    assert_eq!(run(build()), run(build()));
}

fn at_x(g: &ElkGraph, id: &str) -> f64 {
    g.children.iter().find(|n| n.id == id).unwrap().x.unwrap()
}
