//! The heavy algorithms' tests: DISCO, the two SPOrE passes, and
//! Libavoid.

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

fn x_of(g: &ElkGraph, id: &str) -> f64 {
    g.children.iter().find(|n| n.id == id).unwrap().x.unwrap()
}

fn y_of(g: &ElkGraph, id: &str) -> f64 {
    g.children.iter().find(|n| n.id == id).unwrap().y.unwrap()
}

// ---- DISCO ----

#[test]
fn disco_separates_two_components_in_a_row() {
    // two disjoint pairs: each lays out on its own, then the second
    // component sits entirely right of the first
    let mut g = graph_with(
        vec![
            node("a", 40.0, 40.0),
            node("b", 40.0, 40.0),
            node("c", 40.0, 40.0),
            node("d", 40.0, 40.0),
        ],
        vec![edge("ab", "a", "b"), edge("cd", "c", "d")],
        "disco",
    );
    layout::by_name("disco").unwrap().layout(&mut g).unwrap();
    let right_edge_of_first = (x_of(&g, "a") + 40.0).max(x_of(&g, "b") + 40.0);
    let left_edge_of_second = x_of(&g, "c").min(x_of(&g, "d"));
    assert!(
        left_edge_of_second >= right_edge_of_first,
        "the components overlap"
    );
    // the members of a component stay together on one row
    assert_eq!(y_of(&g, "a"), y_of(&g, "b"));
}

#[test]
fn disco_arrangement_column_stacks() {
    let mut g = graph_with(
        vec![
            node("a", 40.0, 40.0),
            node("b", 40.0, 40.0),
            node("c", 40.0, 40.0),
            node("d", 40.0, 40.0),
        ],
        vec![edge("ab", "a", "b"), edge("cd", "c", "d")],
        "disco",
    );
    g.layout_options
        .insert("disco.componentArrangement", "column");
    layout::by_name("disco").unwrap().layout(&mut g).unwrap();
    let bottom_of_first = (y_of(&g, "a") + 40.0).max(y_of(&g, "b") + 40.0);
    let top_of_second = y_of(&g, "c").min(y_of(&g, "d"));
    assert!(top_of_second >= bottom_of_first, "the columns overlap");
}

#[test]
fn disco_component_algorithm_knob_is_honored() {
    let mut g = graph_with(
        vec![
            node("a", 40.0, 40.0),
            node("b", 40.0, 40.0),
            node("c", 40.0, 40.0),
            node("d", 40.0, 40.0),
        ],
        vec![edge("ab", "a", "b"), edge("cd", "c", "d")],
        "disco",
    );
    g.layout_options
        .insert("disco.componentAlgorithm", "random");
    layout::by_name("disco").unwrap().layout(&mut g).unwrap();
    for n in &g.children {
        assert!(n.x.is_some(), "{} was never laid out", n.id);
    }
}

#[test]
fn disco_is_deterministic() {
    let build = || {
        graph_with(
            vec![
                node("a", 40.0, 40.0),
                node("b", 40.0, 40.0),
                node("c", 40.0, 40.0),
                node("d", 40.0, 40.0),
            ],
            vec![edge("ab", "a", "b"), edge("cd", "c", "d")],
            "disco",
        )
    };
    let run = |mut g: ElkGraph| {
        layout::by_name("disco").unwrap().layout(&mut g).unwrap();
        serde_json_to(&g)
    };
    assert_eq!(run(build()), run(build()));
}

// ---- SPOrE Overlap ----

#[test]
fn spore_overlap_pushes_an_overlapping_pair_apart() {
    let mut a = node("a", 40.0, 40.0);
    let mut b = node("b", 40.0, 40.0);
    a.x = Some(100.0);
    a.y = Some(100.0);
    b.x = Some(120.0);
    b.y = Some(100.0);
    let mut g = graph_with(vec![a, b], vec![], "spore-overlap");
    layout::by_name("spore-overlap")
        .unwrap()
        .layout(&mut g)
        .unwrap();
    let centers_apart = (x_of(&g, "b") - x_of(&g, "a")).abs();
    // the minimum center distance is (40 + 40) / 2 + spacing 10 = 50
    assert!(
        centers_apart >= 50.0,
        "the pair still sits {centers_apart} apart"
    );
}

#[test]
fn spore_overlap_leaves_a_clean_layout_alone() {
    let mut a = node("a", 40.0, 40.0);
    let mut b = node("b", 40.0, 40.0);
    a.x = Some(100.0);
    a.y = Some(100.0);
    b.x = Some(200.0);
    b.y = Some(200.0);
    let mut g = graph_with(vec![a, b], vec![], "spore-overlap");
    layout::by_name("spore-overlap")
        .unwrap()
        .layout(&mut g)
        .unwrap();
    assert_eq!(x_of(&g, "b") - x_of(&g, "a"), 100.0);
}

#[test]
fn spore_overlap_is_deterministic() {
    let build = || {
        let mut a = node("a", 40.0, 40.0);
        let mut b = node("b", 40.0, 40.0);
        let mut c = node("c", 40.0, 40.0);
        a.x = Some(0.0);
        a.y = Some(0.0);
        b.x = Some(20.0);
        b.y = Some(0.0);
        c.x = Some(60.0);
        c.y = Some(10.0);
        graph_with(vec![a, b, c], vec![], "spore-overlap")
    };
    let run = |mut g: ElkGraph| {
        layout::by_name("spore-overlap")
            .unwrap()
            .layout(&mut g)
            .unwrap();
        serde_json_to(&g)
    };
    assert_eq!(run(build()), run(build()));
}

// ---- SPOrE Compaction ----

#[test]
fn spore_compaction_closes_the_horizontal_gap() {
    let mut a = node("a", 40.0, 40.0);
    let mut b = node("b", 40.0, 40.0);
    a.x = Some(100.0);
    a.y = Some(100.0);
    b.x = Some(300.0);
    b.y = Some(100.0);
    let mut g = graph_with(vec![a, b], vec![], "spore-compaction");
    layout::by_name("spore-compaction")
        .unwrap()
        .layout(&mut g)
        .unwrap();
    // same row: b pulls in to a's right edge plus the spacing
    assert_eq!(x_of(&g, "b") - x_of(&g, "a"), 40.0 + 10.0);
}

#[test]
fn spore_compaction_sweeps_unrelated_nodes_to_the_boundary() {
    // elkrb's sweep: nothing vertically overlapping above means the
    // boundary is the origin, so b rides up to the top
    let mut a = node("a", 40.0, 40.0);
    let mut b = node("b", 40.0, 40.0);
    a.x = Some(100.0);
    a.y = Some(100.0);
    b.x = Some(300.0);
    b.y = Some(500.0);
    let mut g = graph_with(vec![a, b], vec![], "spore-compaction");
    layout::by_name("spore-compaction")
        .unwrap()
        .layout(&mut g)
        .unwrap();
    assert!(y_of(&g, "b") < y_of(&g, "a"), "b did not sweep up");
}

#[test]
fn spore_compaction_is_deterministic() {
    let build = || {
        let mut a = node("a", 40.0, 40.0);
        let mut b = node("b", 40.0, 40.0);
        let mut c = node("c", 40.0, 40.0);
        a.x = Some(50.0);
        a.y = Some(50.0);
        b.x = Some(250.0);
        b.y = Some(50.0);
        c.x = Some(150.0);
        c.y = Some(150.0);
        graph_with(vec![a, b, c], vec![], "spore-compaction")
    };
    let run = |mut g: ElkGraph| {
        layout::by_name("spore-compaction")
            .unwrap()
            .layout(&mut g)
            .unwrap();
        serde_json_to(&g)
    };
    assert_eq!(run(build()), run(build()));
}

// ---- Libavoid ----

#[test]
fn libavoid_positions_and_routes() {
    let mut g = graph_with(
        vec![node("a", 40.0, 40.0), node("b", 40.0, 40.0)],
        vec![edge("ab", "a", "b")],
        "libavoid",
    );
    layout::by_name("libavoid").unwrap().layout(&mut g).unwrap();
    // unpositioned nodes land on the box grid
    assert!(x_of(&g, "b") > x_of(&g, "a"));
    let s = &g.edges[0].sections[0];
    assert_eq!(s.start_point.x, x_of(&g, "a") + 20.0);
    assert_eq!(s.end_point.x, x_of(&g, "b") + 20.0);
}

#[test]
fn libavoid_keeps_the_section_on_the_final_positions() {
    let mut a = node("a", 40.0, 40.0);
    let mut b = node("b", 40.0, 40.0);
    a.x = Some(0.0);
    a.y = Some(0.0);
    b.x = Some(200.0);
    b.y = Some(0.0);
    let mut g = graph_with(vec![a, b], vec![edge("ab", "a", "b")], "libavoid");
    layout::by_name("libavoid").unwrap().layout(&mut g).unwrap();
    let s = &g.edges[0].sections[0];
    // the direct path: start and end at the centers, on the padded
    // positions (12, 12) and (212, 12)
    assert_eq!((s.start_point.x, s.start_point.y), (32.0, 32.0));
    assert_eq!((s.end_point.x, s.end_point.y), (232.0, 32.0));
}

#[test]
fn libavoid_is_deterministic() {
    let build = || {
        graph_with(
            vec![node("a", 40.0, 40.0), node("b", 40.0, 40.0)],
            vec![edge("ab", "a", "b")],
            "libavoid",
        )
    };
    let run = |mut g: ElkGraph| {
        layout::by_name("libavoid").unwrap().layout(&mut g).unwrap();
        serde_json_to(&g)
    };
    assert_eq!(run(build()), run(build()));
}

#[test]
fn every_registered_algorithm_lays_out_a_small_graph() {
    // the compatibility sweep: every name in the registry runs
    let algorithms = elkr::layout::registry();
    for (name, algorithm) in algorithms.iter() {
        let mut g = graph_with(
            vec![
                node("a", 40.0, 40.0),
                node("b", 40.0, 40.0),
                node("c", 40.0, 40.0),
            ],
            vec![edge("ab", "a", "b")],
            "layered",
        );
        g.layout_options.insert("elk.algorithm", name);
        algorithm
            .layout(&mut g)
            .unwrap_or_else(|e| panic!("{name} failed: {e}"));
        for n in &g.children {
            assert!(n.x.is_some(), "{name} left {} unplaced", n.id);
        }
    }
}
