//! The routing and label tests: orthogonal bends, path centers, and
//! the label pass.

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

fn layered(nodes: Vec<ElkNode>, edges: Vec<ElkEdge>) -> ElkGraph {
    let mut g = ElkGraph {
        id: "root".to_string(),
        children: nodes,
        edges,
        layout_options: Default::default(),
    };
    g.layout_options.insert("elk.algorithm", "layered");
    g
}

#[test]
fn the_orthogonal_option_bends_at_the_midpoint() {
    // layered computes its own coordinates: a's east anchor to b's
    // west anchor, with the two bends at the horizontal midpoint
    let a = node("a", 40.0, 40.0);
    let b = node("b", 40.0, 60.0);
    let mut e = edge("ab", "a", "b");
    e.layout_options.insert("edge.routing", "orthogonal");
    let mut g = layered(vec![a, b], vec![e]);
    layout::run("layered", &mut g).unwrap();
    let mid_x = ((x_of(&g, "a") + 40.0) + x_of(&g, "b")) / 2.0;
    let s = &g.edges[0].sections[0];
    assert_eq!(s.bend_points.len(), 2);
    assert_eq!(s.bend_points[0].x, mid_x);
    assert_eq!(s.bend_points[0].y, y_of(&g, "a") + 20.0);
    assert_eq!(s.bend_points[1].x, mid_x);
    assert_eq!(s.bend_points[1].y, y_of(&g, "b") + 30.0);
}

#[test]
fn plain_edges_route_straight() {
    let a = node("a", 40.0, 40.0);
    let b = node("b", 40.0, 60.0);
    let mut g = layered(vec![a, b], vec![edge("ab", "a", "b")]);
    layout::run("layered", &mut g).unwrap();
    assert!(g.edges[0].sections[0].bend_points.is_empty());
}

#[test]
fn labels_take_their_route_positions() {
    let a = node("a", 40.0, 40.0);
    let b = node("b", 40.0, 60.0);
    let mut e = edge("ab", "a", "b");
    e.labels = vec![elkr::elk::ElkLabel {
        text: "owns".to_string(),
        x: None,
        y: None,
        width: Some(30.0),
        height: Some(14.0),
    }];
    let mut g = layered(vec![a, b], vec![e]);
    layout::run("layered", &mut g).unwrap();
    let label = &g.edges[0].labels[0];
    let x = label.x.expect("the label has no x");
    let y = label.y.expect("the label has no y");
    // the path midpoint sits between the two anchors
    assert!(x > x_of(&g, "a") + 40.0);
    assert!(x < x_of(&g, "b"));
    // elkrb's CENTER placement lifts the label above the anchor line
    // (the anchor rides a's vertical center)
    assert!(y < y_of(&g, "a") + 20.0);
}

#[test]
fn two_labels_stack_without_colliding() {
    let a = node("a", 40.0, 40.0);
    let b = node("b", 40.0, 60.0);
    let label = || elkr::elk::ElkLabel {
        text: "owns".to_string(),
        x: None,
        y: None,
        width: Some(30.0),
        height: Some(14.0),
    };
    let mut e = edge("ab", "a", "b");
    e.labels = vec![label(), label()];
    let mut g = layered(vec![a, b], vec![e]);
    layout::run("layered", &mut g).unwrap();
    let labels = &g.edges[0].labels;
    // the stack climbs away from the line: index 0 sits highest
    assert!(
        labels[0].y.unwrap() < labels[1].y.unwrap(),
        "the stack did not climb"
    );
    assert_eq!(
        labels[1].y.unwrap() - labels[0].y.unwrap(),
        14.0 + 2.0,
        "the stack is one label height plus the gap apart"
    );
}

fn x_of(g: &ElkGraph, id: &str) -> f64 {
    g.children.iter().find(|n| n.id == id).unwrap().x.unwrap()
}

fn y_of(g: &ElkGraph, id: &str) -> f64 {
    g.children.iter().find(|n| n.id == id).unwrap().y.unwrap()
}
