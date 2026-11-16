//! The hierarchical (nested) tests: compound nodes lay their children
//! and size from them.

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

fn graph_with(nodes: Vec<ElkNode>, edges: Vec<ElkEdge>) -> ElkGraph {
    let mut g = ElkGraph {
        id: "root".to_string(),
        children: nodes,
        edges,
        layout_options: Default::default(),
    };
    g.layout_options.insert("elk.algorithm", "layered");
    g
}

fn by_id<'a>(g: &'a ElkGraph, id: &str) -> &'a ElkNode {
    g.children.iter().find(|n| n.id == id).unwrap()
}

#[test]
fn a_compound_node_lays_its_children_and_sizes_from_them() {
    let mut p = node("p", 200.0, 200.0);
    let mut p1 = node("p1", 60.0, 40.0);
    let mut p2 = node("p2", 60.0, 40.0);
    p1.x = Some(0.0);
    p1.y = Some(0.0);
    p2.x = Some(100.0);
    p2.y = Some(0.0);
    p.children = vec![p1, p2];
    let mut g = graph_with(vec![p, node("q", 40.0, 40.0)], vec![edge("pq", "p", "q")]);
    layout::run("layered", &mut g).unwrap();

    let p = by_id(&g, "p");
    // the children keep local coordinates, shifted by the padding;
    // with no inner edges the pair lands in one layered column
    let p1 = &p.children[0];
    let p2 = &p.children[1];
    assert_eq!(p1.x, Some(12.0));
    assert_eq!(p2.x, Some(12.0));
    assert!(p1.y.unwrap() >= 12.0);
    assert!(p2.y.unwrap() > p1.y.unwrap());
    // the parent sizes from the children's bounding box plus the
    // padding on every side
    let min_x = p1.x.unwrap().min(p2.x.unwrap());
    let min_y = p1.y.unwrap().min(p2.y.unwrap());
    let max_x = (p1.x.unwrap() + p1.width.unwrap()).max(p2.x.unwrap() + p2.width.unwrap());
    let max_y = (p1.y.unwrap() + p1.height.unwrap()).max(p2.y.unwrap() + p2.height.unwrap());
    assert_eq!(p.width, Some(max_x - min_x + 24.0));
    assert_eq!(p.height, Some(max_y - min_y + 24.0));
}

#[test]
fn nested_compounds_recurse() {
    // p contains c, and c contains c1: the innermost lays out first
    // and c sizes from c1
    let mut c1 = node("c1", 40.0, 40.0);
    c1.x = Some(0.0);
    c1.y = Some(0.0);
    let mut c = node("c", 100.0, 100.0);
    c.children = vec![c1];
    let mut p = node("p", 200.0, 200.0);
    p.children = vec![c];
    let mut g = graph_with(vec![p], vec![]);
    layout::run("layered", &mut g).unwrap();

    let p = by_id(&g, "p");
    let c = &p.children[0];
    // c sizes from its own child plus the padding, on both levels
    assert_eq!(c.width, Some(40.0 + 24.0));
    assert_eq!(c.height, Some(40.0 + 24.0));
    assert_eq!(c.children[0].x, Some(12.0));
    assert_eq!(c.children[0].y, Some(12.0));
    // p sizes from c
    assert_eq!(p.width, Some(40.0 + 24.0 + 24.0));
}

#[test]
fn flat_graphs_take_the_flat_path() {
    let mut g = graph_with(
        vec![node("a", 40.0, 40.0), node("b", 40.0, 40.0)],
        vec![edge("ab", "a", "b")],
    );
    layout::run("layered", &mut g).unwrap();
    // layered's flat coordinates: b sits one layer along
    assert!(by_id(&g, "b").x.unwrap() > by_id(&g, "a").x.unwrap());
}

#[test]
fn hierarchical_layout_is_deterministic() {
    let build = || {
        let mut p = node("p", 200.0, 200.0);
        let mut p1 = node("p1", 60.0, 40.0);
        let mut p2 = node("p2", 60.0, 40.0);
        p1.x = Some(0.0);
        p1.y = Some(0.0);
        p2.x = Some(100.0);
        p2.y = Some(0.0);
        p.children = vec![p1, p2];
        graph_with(vec![p, node("q", 40.0, 40.0)], vec![edge("pq", "p", "q")])
    };
    let run = |mut g: ElkGraph| {
        layout::run("layered", &mut g).unwrap();
        serde_json_to(&g)
    };
    assert_eq!(run(build()), run(build()));
}

#[test]
fn nested_children_do_not_leak_to_the_surface() {
    let mut p = node("p", 200.0, 200.0);
    p.children = vec![node("p1", 60.0, 40.0)];
    let mut g = graph_with(vec![p, node("q", 40.0, 40.0)], vec![]);
    layout::run("layered", &mut g).unwrap();
    assert_eq!(g.children.len(), 2, "the surface grew");
    assert_eq!(by_id(&g, "p").children.len(), 1);
}
