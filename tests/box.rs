//! The Box algorithm tests: the uniform grid, the aspect ratio knob,
//! and the edge routing.

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

fn graph(nodes: Vec<ElkNode>, edges: Vec<ElkEdge>) -> ElkGraph {
    ElkGraph {
        id: "root".to_string(),
        children: nodes,
        edges,
        layout_options: Default::default(),
    }
}

#[test]
fn places_a_uniform_grid_rows_first() {
    // five 40x40 nodes, aspect 1.6: cols = ceil(sqrt(8)) = 3, so rows
    // of 3 + 2, cells spaced by the node spacing (20)
    let mut g = graph(
        vec![
            node("a", 40.0, 40.0),
            node("b", 40.0, 40.0),
            node("c", 40.0, 40.0),
            node("d", 40.0, 40.0),
            node("e", 40.0, 40.0),
        ],
        vec![],
    );
    layout::by_name("box").unwrap().layout(&mut g).unwrap();
    let at = |i: usize| (g.children[i].x.unwrap(), g.children[i].y.unwrap());
    assert_eq!(at(0), (12.0, 12.0));
    assert_eq!(at(1), (72.0, 12.0));
    assert_eq!(at(2), (132.0, 12.0));
    assert_eq!(at(3), (12.0, 72.0));
    assert_eq!(at(4), (72.0, 72.0));
}

#[test]
fn the_cell_takes_the_largest_node() {
    // one tall node makes every row 60 high, whatever the others are
    let mut g = graph(vec![node("a", 40.0, 40.0), node("b", 40.0, 80.0)], vec![]);
    layout::by_name("box").unwrap().layout(&mut g).unwrap();
    assert_eq!(g.children[1].y.unwrap(), 12.0);
    assert_eq!(g.children[1].x.unwrap(), 12.0 + 40.0 + 20.0);
}

#[test]
fn edges_route_center_to_center() {
    let mut g = graph(
        vec![node("a", 40.0, 40.0), node("b", 40.0, 40.0)],
        vec![edge("ab", "a", "b")],
    );
    layout::by_name("box").unwrap().layout(&mut g).unwrap();
    let s = &g.edges[0].sections[0];
    assert_eq!((s.start_point.x, s.start_point.y), (32.0, 32.0));
    assert_eq!((s.end_point.x, s.end_point.y), (92.0, 32.0));
}

#[test]
fn empty_graph_is_a_noop() {
    let mut g = graph(vec![], vec![]);
    layout::by_name("box").unwrap().layout(&mut g).unwrap();
    assert!(g.children.is_empty());
}
