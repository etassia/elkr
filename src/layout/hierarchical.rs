//! Hierarchical (nested) graphs: the port of elkrb's
//! `HierarchicalProcessor` as layered runs it.
//!
//! A graph is hierarchical when any root child carries children. The
//! pipeline: lay every child group recursively (each group is its own
//! graph with default options, so the recursion bottoms out at flat
//! nodes), run the top-level flat pass, shift each compound node's
//! children in by the padding, and size every compound node from its
//! children's bounding box plus the padding — innermost first.
//!
//! Two faithful quirks worth knowing: the top-level flat pass uses the
//! compound nodes' declared sizes (elkrb sizes parents only after the
//! pass), and children keep coordinates local to their parent rather
//! than absolute. Nested edges do not exist in elkr's model — an edge
//! with nested endpoints routes across the surface as a root edge.

use super::{apply_padding, LayoutError};
use crate::elk::{ElkGraph, ElkNode};

pub const PADDING: f64 = 12.0;

pub fn is_hierarchical(graph: &ElkGraph) -> bool {
    graph.children.iter().any(|n| !n.children.is_empty())
}

/// The elkrb pipeline around an algorithm's flat pass.
pub fn run(
    layer_flat: &impl Fn(&mut ElkGraph) -> Result<(), LayoutError>,
    graph: &mut ElkGraph,
) -> Result<(), LayoutError> {
    layout_children_recursively(layer_flat, graph)?;
    layer_flat(graph)?;
    apply_parent_constraints(graph);
    update_parent_bounds(graph);
    Ok(())
}

fn layout_children_recursively(
    layer_flat: &impl Fn(&mut ElkGraph) -> Result<(), LayoutError>,
    graph: &mut ElkGraph,
) -> Result<(), LayoutError> {
    let compounds: Vec<usize> = graph
        .children
        .iter()
        .enumerate()
        .filter(|(_, n)| !n.children.is_empty())
        .map(|(i, _)| i)
        .collect();
    for i in compounds {
        let mut child_graph = ElkGraph {
            id: format!("{}_children", graph.children[i].id),
            children: std::mem::take(&mut graph.children[i].children),
            edges: Vec::new(),
            layout_options: Default::default(),
        };
        run(layer_flat, &mut child_graph)?;
        let laid = child_graph.children;
        graph.children[i].children = laid;
    }
    Ok(())
}

fn apply_parent_constraints(graph: &mut ElkGraph) {
    for node in graph.children.iter_mut() {
        if node.children.is_empty() {
            continue;
        }
        for child in &mut node.children {
            child.x = Some(child.x.unwrap_or(0.0) + PADDING);
            child.y = Some(child.y.unwrap_or(0.0) + PADDING);
        }
    }
}

fn update_parent_bounds(graph: &mut ElkGraph) {
    for index in 0..graph.children.len() {
        if graph.children[index].children.is_empty() {
            continue;
        }
        // innermost first: recurse through an owned view
        let mut inner = std::mem::take(&mut graph.children[index].children);
        {
            let mut inner_graph = ElkGraph {
                id: format!("{}_bounds", graph.children[index].id),
                children: std::mem::take(&mut inner),
                edges: Vec::new(),
                layout_options: Default::default(),
            };
            update_parent_bounds(&mut inner_graph);
            inner = inner_graph.children;
        }
        let (min_x, min_y, width, height) = bounds(&inner);
        // elkrb shifts only when the children's minimum went negative:
        // the constraint pass already moved positive layouts in
        let offset_x = if min_x < 0.0 { -min_x + PADDING } else { 0.0 };
        let offset_y = if min_y < 0.0 { -min_y + PADDING } else { 0.0 };
        for child in inner.iter_mut() {
            child.x = Some(child.x.unwrap_or(0.0) + offset_x);
            child.y = Some(child.y.unwrap_or(0.0) + offset_y);
        }
        let node = &mut graph.children[index];
        node.children = inner;
        node.width = Some(width + 2.0 * PADDING);
        node.height = Some(height + 2.0 * PADDING);
    }
}

fn bounds(children: &[ElkNode]) -> (f64, f64, f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for child in children {
        let x = child.x.unwrap_or(0.0);
        let y = child.y.unwrap_or(0.0);
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x + child.width.unwrap_or(0.0));
        max_y = max_y.max(y + child.height.unwrap_or(0.0));
    }
    if children.is_empty() {
        return (0.0, 0.0, 0.0, 0.0);
    }
    (min_x, min_y, max_x - min_x, max_y - min_y)
}

/// The padding shift elkrb applies before the flat pass re-runs — kept
/// for parity with `apply_padding` callers.
pub fn pad(graph: &mut ElkGraph) {
    apply_padding(graph);
}
