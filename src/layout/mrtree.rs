//! The MRTree algorithm: the port of elkrb's `MRTree`.
//!
//! Trees grow from the nodes no edge points at (all nodes when every
//! node has an incoming edge): children follow outgoing edges in edge
//! order, leaves sit at their slot and parents center over their
//! children, levels 80 apart, trees advancing by their width plus the
//! node spacing. One deviation, deliberate: a node already placed in a
//! tree is never re-added, so a cyclic graph lays out instead of
//! recursing forever the way elkrb does.

use super::{apply_padding, straight_sections, LayoutAlgorithm, LayoutError};
use crate::elk::ElkGraph;
use std::collections::BTreeSet;

pub struct MRTree;

const LEVEL_HEIGHT: f64 = 80.0;

/// The tree shape, held by node index so the layout pass can borrow
/// the graph mutably.
struct Tree {
    index: usize,
    children: Vec<Tree>,
}

impl LayoutAlgorithm for MRTree {
    fn name(&self) -> &'static str {
        "mrtree"
    }

    fn layout(&self, graph: &mut ElkGraph) -> Result<(), LayoutError> {
        if graph.children.is_empty() {
            return Ok(());
        }

        let spacing = graph.layout_options.node_spacing();

        let mut has_incoming: BTreeSet<&str> = BTreeSet::new();
        for e in &graph.edges {
            for t in &e.targets {
                has_incoming.insert(t.as_str());
            }
        }
        let mut roots: Vec<usize> = graph
            .children
            .iter()
            .enumerate()
            .filter(|(_, n)| !has_incoming.contains(n.id.as_str()))
            .map(|(i, _)| i)
            .collect();
        if roots.is_empty() {
            roots = (0..graph.children.len()).collect();
        }

        let mut built: BTreeSet<usize> = BTreeSet::new();
        let trees: Vec<Tree> = roots
            .iter()
            .map(|&root| build_tree(root, graph, &mut built))
            .collect();

        let mut x_offset = 0.0;
        for tree in &trees {
            layout_tree(tree, x_offset, 0.0, 0, spacing, graph);
            x_offset += tree_width(tree, graph) + spacing;
        }

        apply_padding(graph);
        straight_sections(graph)
    }
}

fn build_tree(index: usize, graph: &ElkGraph, built: &mut BTreeSet<usize>) -> Tree {
    built.insert(index);
    let mut children: Vec<Tree> = Vec::new();
    for e in &graph.edges {
        if !e
            .sources
            .iter()
            .any(|s| s == graph.children[index].id.as_str())
        {
            continue;
        }
        for t in &e.targets {
            let Some(child) = graph.children.iter().position(|n| n.id == *t) else {
                continue;
            };
            if built.contains(&child) {
                continue;
            }
            children.push(build_tree(child, graph, built));
        }
    }
    Tree { index, children }
}

/// Lays the tree out and returns its width plus the spacing (elkrb's
/// `layout_tree` return).
fn layout_tree(
    tree: &Tree,
    x_offset: f64,
    y_offset: f64,
    level: usize,
    spacing: f64,
    graph: &mut ElkGraph,
) -> f64 {
    if tree.children.is_empty() {
        let node = &mut graph.children[tree.index];
        node.x = Some(x_offset);
        node.y = Some(y_offset + level as f64 * LEVEL_HEIGHT);
        return node.width.unwrap_or(0.0) + spacing;
    }

    let mut child_x = x_offset;
    for child in &tree.children {
        let width = layout_tree(child, child_x, y_offset, level + 1, spacing, graph);
        child_x += width;
    }

    let first_index = tree.children[0].index;
    let last_index = tree.children[tree.children.len() - 1].index;
    let first_x = graph.children[first_index].x.unwrap_or(0.0);
    let last_x = graph.children[last_index].x.unwrap_or(0.0);
    let last_width = graph.children[last_index].width.unwrap_or(0.0);
    let center_x = (first_x + last_x + last_width) / 2.0;
    let node = &mut graph.children[tree.index];
    node.x = Some(center_x - node.width.unwrap_or(0.0) / 2.0);
    node.y = Some(y_offset + level as f64 * LEVEL_HEIGHT);

    last_x + last_width - x_offset + spacing
}

fn tree_width(tree: &Tree, graph: &ElkGraph) -> f64 {
    if tree.children.is_empty() {
        return graph.children[tree.index].width.unwrap_or(0.0);
    }
    tree.children.iter().map(|c| tree_width(c, graph)).sum()
}
