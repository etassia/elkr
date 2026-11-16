//! The arrangement algorithms' tests: VertiFlex, Radial, RectPacking
//! and TopdownPacking.

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
        ports: Vec::new(),
    }
}

fn graph_with(nodes: Vec<ElkNode>, algorithm: &str) -> ElkGraph {
    let mut g = ElkGraph {
        id: "root".to_string(),
        children: nodes,
        edges: Vec::new(),
        layout_options: Default::default(),
    };
    g.layout_options.insert("elk.algorithm", algorithm);
    g
}

fn run(algorithm: &str, nodes: Vec<ElkNode>) -> ElkGraph {
    let mut g = graph_with(nodes, algorithm);
    layout::by_name(algorithm).unwrap().layout(&mut g).unwrap();
    g
}

fn at(g: &ElkGraph, id: &str) -> (f64, f64) {
    let n = g.children.iter().find(|n| n.id == id).unwrap();
    (n.x.unwrap(), n.y.unwrap())
}

// ---- VertiFlex ----

#[test]
fn vertiflex_stacks_nodes_into_three_columns() {
    // six 40x40 nodes round-robin into 3 columns of 2; default vertical
    // spacing 30 stacks them at y 12 and 82, columns 90 apart (column
    // width 40 + columnSpacing 50)
    let g = run(
        "vertiflex",
        (0..6).map(|i| node(&format!("n{i}"), 40.0, 40.0)).collect(),
    );
    assert_eq!(at(&g, "n0"), (12.0, 12.0));
    assert_eq!(at(&g, "n1"), (102.0, 12.0));
    assert_eq!(at(&g, "n2"), (192.0, 12.0));
    assert_eq!(at(&g, "n3"), (12.0, 82.0));
    assert_eq!(at(&g, "n4"), (102.0, 82.0));
    assert_eq!(at(&g, "n5"), (192.0, 82.0));
}

#[test]
fn vertiflex_column_count_is_a_knob() {
    let mut two_columns = graph_with(
        vec![
            node("a", 40.0, 40.0),
            node("b", 40.0, 40.0),
            node("c", 40.0, 40.0),
            node("d", 40.0, 40.0),
        ],
        "vertiflex",
    );
    two_columns
        .layout_options
        .insert("vertiflex.columnCount", "2");
    layout::by_name("vertiflex")
        .unwrap()
        .layout(&mut two_columns)
        .unwrap();
    // four nodes in two balanced columns: a and c share column one
    assert_eq!(at(&two_columns, "a").0, at(&two_columns, "c").0);
    assert_eq!(at(&two_columns, "b").0, at(&two_columns, "d").0);
    assert_ne!(at(&two_columns, "a").0, at(&two_columns, "b").0);
}

#[test]
fn vertiflex_single_node_sits_at_the_padding() {
    let g = run("vertiflex", vec![node("a", 40.0, 40.0)]);
    assert_eq!(at(&g, "a"), (12.0, 12.0));
}

#[test]
fn vertiflex_is_deterministic() {
    let run_once = || {
        let g = run(
            "vertiflex",
            (0..5).map(|i| node(&format!("n{i}"), 40.0, 40.0)).collect(),
        );
        serde_json_to(&g)
    };
    assert_eq!(run_once(), run_once());
}

// ---- Radial ----

#[test]
fn radial_places_even_angles_on_one_circle() {
    let g = run(
        "radial",
        vec![
            node("a", 40.0, 40.0),
            node("b", 40.0, 40.0),
            node("c", 40.0, 40.0),
        ],
    );
    // a regular polygon's vertices sit equidistant from the centroid,
    // and the padding shifts every vertex by the same offset
    let centers: Vec<(f64, f64)> = g
        .children
        .iter()
        .map(|n| (n.x.unwrap() + 20.0, n.y.unwrap() + 20.0))
        .collect();
    let centroid = (
        centers.iter().map(|c| c.0).sum::<f64>() / 3.0,
        centers.iter().map(|c| c.1).sum::<f64>() / 3.0,
    );
    let distances: Vec<f64> = centers
        .iter()
        .map(|(x, y)| ((x - centroid.0).powi(2) + (y - centroid.1).powi(2)).sqrt())
        .collect();
    let first = distances[0];
    for d in &distances {
        assert!(
            (d - first).abs() < 0.001,
            "the nodes are not on one circle: {distances:?}"
        );
    }
}

#[test]
fn radial_single_node_sits_at_the_padding() {
    let g = run("radial", vec![node("a", 40.0, 40.0)]);
    assert_eq!(at(&g, "a"), (12.0, 12.0));
}

#[test]
fn radial_is_deterministic() {
    let run_once = || {
        let g = run(
            "radial",
            (0..7).map(|i| node(&format!("n{i}"), 40.0, 40.0)).collect(),
        );
        serde_json_to(&g)
    };
    assert_eq!(run_once(), run_once());
}

// ---- RectPacking ----

#[test]
fn rectpacking_lines_the_nodes_up_on_one_shelf() {
    // tallest-first puts the shelf height at the maximum placed so
    // far, so every later node passes the 80% rule and joins the same
    // shelf — elkrb's own behavior; the row orders by height
    let g = run(
        "rectpacking",
        vec![
            node("tall", 40.0, 100.0),
            node("mid", 40.0, 90.0),
            node("huge", 40.0, 200.0),
        ],
    );
    assert_eq!(at(&g, "huge").1, at(&g, "tall").1);
    assert_eq!(at(&g, "mid").1, at(&g, "tall").1);
    assert_eq!(at(&g, "huge").0, 12.0);
    assert_eq!(at(&g, "tall").0, 12.0 + 40.0 + 20.0);
    assert_eq!(at(&g, "mid").0, 12.0 + 2.0 * (40.0 + 20.0));
}

#[test]
fn rectpacking_short_nodes_share_a_tall_shelf() {
    // the uniformity rule keeps short nodes on a tall shelf
    let g = run(
        "rectpacking",
        vec![
            node("tall", 40.0, 100.0),
            node("mid", 40.0, 90.0),
            node("short", 40.0, 20.0),
        ],
    );
    assert_eq!(at(&g, "tall").1, at(&g, "short").1);
}

#[test]
fn rectpacking_is_deterministic() {
    let run_once = || {
        let g = run(
            "rectpacking",
            (0..6)
                .map(|i| {
                    node(
                        &format!("n{i}"),
                        40.0 + (i as f64) * 10.0,
                        30.0 + (i as f64) * 7.0,
                    )
                })
                .collect(),
        );
        serde_json_to(&g)
    };
    assert_eq!(run_once(), run_once());
}

// ---- TopdownPacking ----

#[test]
fn topdownpacking_builds_a_uniform_square_grid() {
    // nine 40x40 nodes: cols = 3, every node resized to the average
    // cell (40x40) and placed on a 60-pitch grid
    let g = run(
        "topdownpacking",
        (0..9).map(|i| node(&format!("n{i}"), 40.0, 40.0)).collect(),
    );
    for n in &g.children {
        assert_eq!((n.width, n.height), (Some(40.0), Some(40.0)));
    }
    assert_eq!(at(&g, "n0"), (12.0, 12.0));
    assert_eq!(at(&g, "n2"), (132.0, 12.0));
    assert_eq!(at(&g, "n3"), (12.0, 72.0));
    assert_eq!(at(&g, "n8"), (132.0, 132.0));
}

#[test]
fn topdownpacking_uniform_cell_from_the_average() {
    // mixed sizes: the average width and height shape every cell
    let g = run(
        "topdownpacking",
        vec![
            node("a", 40.0, 40.0),
            node("b", 60.0, 60.0),
            node("c", 20.0, 20.0),
            node("d", 40.0, 40.0),
        ],
    );
    for n in &g.children {
        assert_eq!((n.width, n.height), (Some(40.0), Some(40.0)));
    }
}

#[test]
fn topdownpacking_is_deterministic() {
    let run_once = || {
        let g = run(
            "topdownpacking",
            (0..5).map(|i| node(&format!("n{i}"), 40.0, 40.0)).collect(),
        );
        serde_json_to(&g)
    };
    assert_eq!(run_once(), run_once());
}

#[test]
fn the_geometry_algorithms_skip_unknown_edge_endpoints() {
    // elkrb's EdgeRouter passes over edges it cannot resolve; the
    // geometry algorithms inherit that, unlike layered and force
    for algorithm in ["vertiflex", "radial", "rectpacking", "topdownpacking"] {
        let mut g = graph_with(vec![node("a", 40.0, 40.0)], algorithm);
        g.edges = vec![elkr::elk::ElkEdge {
            id: "bad".to_string(),
            sources: vec!["a".to_string()],
            targets: vec!["ghost".to_string()],
            sections: Vec::new(),
        }];
        layout::by_name(algorithm).unwrap().layout(&mut g).unwrap();
        assert!(
            g.edges[0].sections.is_empty(),
            "{algorithm} routed an edge with an unknown endpoint"
        );
    }
}
