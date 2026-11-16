//! The Random algorithm tests: the deterministic scatter, the bounds
//! it scatters inside, and the aspect ratio knob.

use elkr::elk::{ElkGraph, ElkNode};
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
    }
}

fn graph_with(nodes: Vec<ElkNode>, key: Option<(&str, &str)>) -> ElkGraph {
    let mut g = ElkGraph {
        id: "root".to_string(),
        children: nodes,
        edges: Vec::new(),
        layout_options: Default::default(),
    };
    if let Some((k, v)) = key {
        g.layout_options.insert(k, v);
    }
    g
}

fn random_five() -> ElkGraph {
    graph_with(
        vec![
            node("a", 40.0, 40.0),
            node("b", 40.0, 40.0),
            node("c", 40.0, 40.0),
            node("d", 40.0, 40.0),
            node("e", 40.0, 40.0),
        ],
        Some(("elk.algorithm", "random")),
    )
}

#[test]
fn same_input_same_output() {
    let run = || {
        let mut g = random_five();
        layout::by_name("random").unwrap().layout(&mut g).unwrap();
        serde_json_to(&g)
    };
    assert_eq!(run(), run(), "the random scatter must be byte-identical");
}

#[test]
fn the_scatter_spreads_inside_the_area_square() {
    let mut g = random_five();
    layout::by_name("random").unwrap().layout(&mut g).unwrap();
    // five 40x40 nodes, spacing 20: area = 5 * 60 * 60 = 18000;
    // width = sqrt(18000 * 1.6) ~= 169.7, height ~= 106.1; each node's
    // x range is width - 40, y range height - 40, plus the padding
    let width = (18000.0f64 * 1.6).sqrt();
    let height = width / 1.6;
    for n in &g.children {
        let x = n.x.unwrap();
        let y = n.y.unwrap();
        assert!(
            x >= 12.0 && x <= 12.0 + width - 40.0,
            "x {x} outside the scatter bounds"
        );
        assert!(
            y >= 12.0 && y <= 12.0 + height - 40.0,
            "y {y} outside the scatter bounds"
        );
    }
}

#[test]
fn nodes_do_not_share_a_position() {
    let mut g = random_five();
    layout::by_name("random").unwrap().layout(&mut g).unwrap();
    let mut spots: Vec<(u64, u64)> = g
        .children
        .iter()
        .map(|n| {
            (
                (n.x.unwrap() * 1000.0) as u64,
                (n.y.unwrap() * 1000.0) as u64,
            )
        })
        .collect();
    spots.sort();
    spots.dedup();
    assert_eq!(spots.len(), 5, "two nodes landed on the same spot");
}

#[test]
fn the_aspect_ratio_knob_changes_the_bounds() {
    let mut wide = graph_with(
        vec![
            node("a", 40.0, 40.0),
            node("b", 40.0, 40.0),
            node("c", 40.0, 40.0),
        ],
        Some(("elk.algorithm", "random")),
    );
    let mut square = graph_with(
        vec![
            node("a", 40.0, 40.0),
            node("b", 40.0, 40.0),
            node("c", 40.0, 40.0),
        ],
        Some(("elk.algorithm", "random")),
    );
    square.layout_options.insert("aspect_ratio", "1");

    layout::by_name("random")
        .unwrap()
        .layout(&mut wide)
        .unwrap();
    layout::by_name("random")
        .unwrap()
        .layout(&mut square)
        .unwrap();
    assert_ne!(
        serde_json_to(&wide.children),
        serde_json_to(&square.children),
        "the aspect ratio did not reach the scatter"
    );
}

#[test]
fn empty_graph_is_a_noop() {
    let mut g = graph_with(vec![], Some(("elk.algorithm", "random")));
    layout::by_name("random").unwrap().layout(&mut g).unwrap();
    assert!(g.children.is_empty());
}
