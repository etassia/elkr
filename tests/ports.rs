//! The port constraint tests: side detection, ordering, distribution,
//! and the port-anchored edge sections.

use elkr::elk::{ElkEdge, ElkGraph, ElkNode, ElkPort};
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

fn port(id: &str, x: f64, y: f64) -> ElkPort {
    ElkPort {
        id: id.to_string(),
        x,
        y,
        width: None,
        height: None,
        side: "UNDEFINED".to_string(),
        index: -1,
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
fn the_constraint_pass_detects_sides_and_distributes() {
    let mut a = node("a", 40.0, 40.0);
    // p1 sits on the top edge, p2 on the right edge
    a.ports = vec![port("p1", 20.0, 0.0), port("p2", 40.0, 20.0)];
    let mut g = layered(vec![a], vec![]);
    layout::run("layered", &mut g).unwrap();
    let ports = &g.children[0].ports;
    assert_eq!(ports[0].side, "NORTH");
    assert_eq!(ports[0].y, 0.0);
    // one port on a side takes the side's midpoint
    assert_eq!(ports[0].x, 20.0);
    assert_eq!(ports[1].side, "EAST");
    assert_eq!(ports[1].x, 40.0);
    assert_eq!(ports[1].y, 20.0);
}

#[test]
fn explicit_index_orders_the_side() {
    let mut a = node("a", 60.0, 40.0);
    let mut late = port("late", 10.0, 0.0);
    let mut early = port("early", 30.0, 0.0);
    let mut mid = port("mid", 50.0, 0.0);
    late.index = 2;
    early.index = 0;
    mid.index = 1;
    a.ports = vec![late, early, mid];
    let mut g = layered(vec![a], vec![]);
    layout::run("layered", &mut g).unwrap();
    let ports = &g.children[0].ports;
    // spread along the north edge in index order: early, mid, late
    assert!(
        ports.iter().find(|p| p.id == "early").unwrap().x
            < ports.iter().find(|p| p.id == "mid").unwrap().x
    );
    assert!(
        ports.iter().find(|p| p.id == "mid").unwrap().x
            < ports.iter().find(|p| p.id == "late").unwrap().x
    );
    assert_eq!(ports.iter().find(|p| p.id == "early").unwrap().x, 15.0);
    assert_eq!(ports.iter().find(|p| p.id == "mid").unwrap().x, 30.0);
    assert_eq!(ports.iter().find(|p| p.id == "late").unwrap().x, 45.0);
}

#[test]
fn layered_anchors_edges_at_named_ports() {
    // a -> p2, where p2 is b's west-side port: the section ends at the
    // port, not b's border midpoint
    let a = node("a", 40.0, 40.0);
    let mut b = node("b", 40.0, 40.0);
    let mut p2 = port("p2", 0.0, 10.0);
    p2.side = "WEST".to_string();
    b.ports = vec![p2];
    let mut g = layered(vec![a, b], vec![edge("ab", "a", "p2")]);
    layout::run("layered", &mut g).unwrap();
    let s = &g.edges[0].sections[0];
    // b lands at (92, 12) in the two-layer layout; the constraint pass
    // redistributes the declared port onto the west edge's even spread,
    // so the single west port sits at the edge's midpoint
    let bx = g.children[1].x.unwrap();
    let by = g.children[1].y.unwrap();
    assert_eq!((s.end_point.x, s.end_point.y), (bx, by + 20.0));
    assert_eq!(s.outgoing_shape.as_deref(), Some("b"));
}

#[test]
fn straight_sections_anchor_at_ports() {
    let mut a = node("a", 40.0, 40.0);
    let mut b = node("b", 40.0, 40.0);
    let mut p1 = port("p1", 40.0, 20.0);
    p1.side = "EAST".to_string();
    let mut p2 = port("p2", 0.0, 20.0);
    p2.side = "WEST".to_string();
    a.ports = vec![p1];
    b.ports = vec![p2];
    let mut g = layered(vec![a, b], vec![edge("ab", "p1", "p2")]);
    g.layout_options.insert("elk.algorithm", "box");
    layout::run("box", &mut g).unwrap();
    let s = &g.edges[0].sections[0];
    assert_eq!(s.start_point.y, y_of(&g, "a") + 20.0);
    assert_eq!(s.end_point.y, y_of(&g, "b") + 20.0);
    assert_eq!(s.start_point.x, x_of(&g, "a") + 40.0);
    assert_eq!(s.end_point.x, x_of(&g, "b"));
}

fn x_of(g: &ElkGraph, id: &str) -> f64 {
    g.children.iter().find(|n| n.id == id).unwrap().x.unwrap()
}

fn y_of(g: &ElkGraph, id: &str) -> f64 {
    g.children.iter().find(|n| n.id == id).unwrap().y.unwrap()
}

#[test]
fn port_endpoints_resolve_for_layering() {
    // an edge naming a port lays out without the unknown-endpoint error
    let a = node("a", 40.0, 40.0);
    let mut b = node("b", 40.0, 40.0);
    let mut p2 = port("p2", 0.0, 20.0);
    p2.side = "WEST".to_string();
    b.ports = vec![p2];
    let mut g = layered(vec![a, b], vec![edge("ab", "a", "p2")]);
    layout::run("layered", &mut g).unwrap();
    assert!(g.children[1].x.is_some());
}

#[test]
fn ports_round_trip_through_json() {
    let mut a = node("a", 40.0, 40.0);
    a.ports = vec![port("p1", 20.0, 0.0)];
    let mut g = layered(vec![a], vec![]);
    layout::run("layered", &mut g).unwrap();
    let json = serde_json::to_string(&g).unwrap();
    let back: ElkGraph = serde_json::from_str(&json).unwrap();
    assert_eq!(back.children[0].ports[0].id, "p1");
    assert_eq!(back.children[0].ports[0].side, "NORTH");
}
