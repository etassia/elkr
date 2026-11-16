//! The Fixed algorithm tests: presets keep their relative geometry and
//! the bounding box lands on the padding.

use elkr::elk::{ElkEdge, ElkGraph, ElkNode};
use elkr::layout;

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

fn edge(id: &str, src: &str, tgt: &str) -> ElkEdge {
    ElkEdge {
        id: id.to_string(),
        sources: vec![src.to_string()],
        targets: vec![tgt.to_string()],
        sections: Vec::new(),
    }
}

fn graph(nodes: Vec<ElkNode>, edges: Vec<ElkEdge>) -> ElkGraph {
    ElkGraph {
        id: "root".to_string(),
        children: nodes,
        edges,
        layout_options: Default::default(),
    }
}

#[test]
fn preset_positions_keep_their_offsets() {
    let mut a = node("a", 40.0, 40.0);
    let mut b = node("b", 40.0, 40.0);
    a.x = Some(100.0);
    a.y = Some(50.0);
    b.x = Some(200.0);
    b.y = Some(-30.0);
    let mut g = graph(vec![a, b], vec![]);
    layout::by_name("fixed").unwrap().layout(&mut g).unwrap();
    // the bounding box min (100, -30) shifts to (12, 12)
    assert_eq!((g.children[0].x, g.children[0].y), (Some(12.0), Some(92.0)));
    assert_eq!(
        (g.children[1].x, g.children[1].y),
        (Some(112.0), Some(12.0))
    );
}

#[test]
fn absent_coordinates_become_the_origin() {
    let mut g = graph(vec![node("a", 40.0, 40.0)], vec![]);
    layout::by_name("fixed").unwrap().layout(&mut g).unwrap();
    assert_eq!((g.children[0].x, g.children[0].y), (Some(12.0), Some(12.0)));
}

#[test]
fn edges_route_center_to_center() {
    let mut a = node("a", 40.0, 40.0);
    let mut b = node("b", 40.0, 40.0);
    a.x = Some(0.0);
    a.y = Some(0.0);
    b.x = Some(100.0);
    b.y = Some(0.0);
    let mut g = graph(vec![a, b], vec![edge("ab", "a", "b")]);
    layout::by_name("fixed").unwrap().layout(&mut g).unwrap();
    let s = &g.edges[0].sections[0];
    assert_eq!((s.start_point.x, s.start_point.y), (32.0, 32.0));
    assert_eq!((s.end_point.x, s.end_point.y), (132.0, 32.0));
}

#[test]
fn empty_graph_is_a_noop() {
    let mut g = graph(vec![], vec![]);
    layout::by_name("fixed").unwrap().layout(&mut g).unwrap();
    assert!(g.children.is_empty());
}
