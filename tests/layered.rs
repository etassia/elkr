use elkr::{apply, ElkEdge, ElkGraph, ElkNode};
use serde_json::json;

fn node(id: &str, w: f64, h: f64) -> ElkNode {
    ElkNode {
        id: id.into(),
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
        id: id.into(),
        sources: vec![src.into()],
        targets: vec![tgt.into()],
        sections: Vec::new(),
    }
}

fn diamond() -> ElkGraph {
    // a -> b -> d ; a -> c -> d  (the classic two-path diamond)
    ElkGraph {
        id: "root".into(),
        children: vec![
            node("a", 100.0, 40.0),
            node("b", 80.0, 40.0),
            node("c", 80.0, 40.0),
            node("d", 100.0, 40.0),
        ],
        edges: vec![
            edge("e1", "a", "b"),
            edge("e2", "a", "c"),
            edge("e3", "b", "d"),
            edge("e4", "c", "d"),
        ],
        layout_options: Default::default(),
    }
}

#[test]
fn layers_follow_longest_path() {
    let mut g = diamond();
    apply(&mut g).expect("layout");
    let (ax, _by) = (
        g.find_node("a").unwrap().x.unwrap(),
        g.find_node("a").unwrap().y.unwrap(),
    );
    let dx = g.find_node("d").unwrap().x.unwrap();
    let bx = g.find_node("b").unwrap().x.unwrap();
    // a in layer 0, b and c in layer 1, d in layer 2
    assert!(ax < bx && bx < dx);
    // b and c share the layer (same x)
    let cx = g.find_node("c").unwrap().x.unwrap();
    assert_eq!(bx, cx);
}

#[test]
fn layers_stack_across_with_spacing() {
    let mut g = diamond();
    apply(&mut g).expect("layout");
    // default node spacing 20: b at y 0, c at y 40+20
    let by = g.find_node("b").unwrap().y.unwrap();
    let cy = g.find_node("c").unwrap().y.unwrap();
    assert_eq!(by, 0.0);
    assert_eq!(cy, 60.0);
}

#[test]
fn layer_spacing_separates_layers() {
    let mut g = diamond();
    g.layout_options
        .insert("elk.layered.spacing.nodeNodeBetweenLayers", "100");
    apply(&mut g).expect("layout");
    // a width 100 + layer gap 100 -> layer 1 at x 200
    let bx = g.find_node("b").unwrap().x.unwrap();
    assert_eq!(bx, 200.0);
}

#[test]
fn same_input_same_output() {
    let mut a = diamond();
    let mut b = diamond();
    apply(&mut a).expect("layout a");
    apply(&mut b).expect("layout b");
    let sa = serde_json::to_string(&a).unwrap();
    let sb = serde_json_to(&b);
    assert_eq!(sa, sb);
}

fn serde_json_to(g: &ElkGraph) -> String {
    serde_json::to_string(g).unwrap()
}

#[test]
fn down_direction_transposes_axes() {
    let mut g = diamond();
    g.layout_options.insert("elk.direction", "DOWN");
    apply(&mut g).expect("layout");
    let ay = g.find_node("a").unwrap().y.unwrap();
    let dy = g.find_node("d").unwrap().y.unwrap();
    let bx = g.find_node("b").unwrap().x.unwrap();
    let cx = g.find_node("c").unwrap().x.unwrap();
    // layers run along y; the middle layer's nodes stack along x
    assert!(ay < dy);
    assert!(bx < cx);
    let ax = g.find_node("a").unwrap().x.unwrap();
    assert_eq!(ax, bx);
}

#[test]
fn left_direction_mirrors() {
    let mut right = diamond();
    apply(&mut right).expect("layout right");
    let mut left = diamond();
    left.layout_options.insert("elk.direction", "LEFT");
    apply(&mut left).expect("layout left");
    // the source sits furthest along +x under LEFT, under the mirror
    let ax = left.find_node("a").unwrap().x.unwrap();
    let dx = left.find_node("d").unwrap().x.unwrap();
    assert!(ax > dx);
    let _ = right;
}

#[test]
fn edges_route_between_node_sides() {
    let mut g = diamond();
    apply(&mut g).expect("layout");
    let a = g.find_node("a").unwrap();
    let (ax, ay, aw, ah) = (
        a.x.unwrap(),
        a.y.unwrap(),
        a.width.unwrap(),
        a.height.unwrap(),
    );
    let e = g.edges.iter().find(|e| e.id == "e1").unwrap();
    let s = &e.sections[0];
    // e1: a -> b, a's source anchor is its right edge center
    assert_eq!(s.start_point.x, ax + aw);
    assert_eq!(s.start_point.y, ay + ah / 2.0);
    let b = g.find_node("b").unwrap();
    let (bx, bh) = (b.x.unwrap(), b.height.unwrap());
    let bby = b.y.unwrap();
    // b's target anchor is its left edge center
    assert_eq!(s.end_point.x, bx);
    assert_eq!(s.end_point.y, bby + bh / 2.0);
    assert_eq!(s.incoming_shape.as_deref(), Some("a"));
    assert_eq!(s.outgoing_shape.as_deref(), Some("b"));
}

#[test]
fn cycles_reverse_for_layering_only() {
    // a -> b -> c -> a: every node lands in its own layer, sections keep
    // the authored direction
    let mut g = ElkGraph {
        id: "root".into(),
        children: vec![
            node("a", 40.0, 40.0),
            node("b", 40.0, 40.0),
            node("c", 40.0, 40.0),
        ],
        edges: vec![
            edge("e1", "a", "b"),
            edge("e2", "b", "c"),
            edge("e3", "c", "a"),
        ],
        layout_options: Default::default(),
    };
    apply(&mut g).expect("layout");
    let xs: Vec<f64> = ["a", "b", "c"]
        .iter()
        .map(|id| g.find_node(id).unwrap().x.unwrap())
        .collect();
    // three distinct layers along x
    let mut sorted = xs.clone();
    sorted.sort_by(|p, q| p.partial_cmp(q).unwrap());
    assert!(sorted[1] - sorted[0] >= 60.0);
    assert!(sorted[2] - sorted[1] >= 60.0);
    let e3 = g.edges.iter().find(|e| e.id == "e3").unwrap();
    // c -> a keeps its authored direction in the section
    assert_eq!(e3.sections[0].incoming_shape.as_deref(), Some("c"));
    assert_eq!(e3.sections[0].outgoing_shape.as_deref(), Some("a"));
}

#[test]
fn eljs_style_json_round_trips() {
    let input = json!({
        "id": "root",
        "children": [
            {"id": "n1", "width": 120, "height": 60},
            {"id": "n2", "width": 120, "height": 60}
        ],
        "edges": [{"id": "e1", "sources": ["n1"], "targets": ["n2"]}],
        "layoutOptions": {"elk.direction": "RIGHT"}
    });
    let mut g: ElkGraph = serde_json::from_value(input).expect("parse");
    apply(&mut g).expect("layout");
    assert_eq!(g.find_node("n1").unwrap().x, Some(0.0));
    let n2 = g.find_node("n2").unwrap();
    assert_eq!(n2.x, Some(180.0)); // 120 width + 60 default layer spacing
    let text = serde_json::to_string(&g).unwrap();
    assert!(text.contains("\"layoutOptions\":{\"elk.direction\":\"RIGHT\"}"));
}

#[test]
fn unknown_edge_endpoint_is_an_error() {
    let mut g = ElkGraph {
        id: "root".into(),
        children: vec![node("a", 40.0, 40.0)],
        edges: vec![edge("e1", "a", "ghost")],
        layout_options: Default::default(),
    };
    let err = apply(&mut g).expect_err("must fail");
    assert!(err.to_string().contains("ghost"));
}
