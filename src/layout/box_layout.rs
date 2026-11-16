//! The Box algorithm: the port of elkrb's `Box`.
//!
//! Nodes go into a uniform grid, rows left to right and top to bottom.
//! The column count approximates the `aspect_ratio` option (1.6 by
//! default): cols = ceil(sqrt(n * ratio)). Every cell shares the
//! largest node's width and height, spaced by
//! `elk.spacing.nodeNode`.

use super::{apply_padding, straight_sections, LayoutAlgorithm, LayoutError};
use crate::elk::{ElkGraph, DEFAULT_NODE_HEIGHT, DEFAULT_NODE_WIDTH};
use std::str::FromStr;

pub struct BoxLayout;

fn option<T: FromStr>(graph: &ElkGraph, bare: &str, default: T) -> T {
    graph
        .layout_options
        .get_parsed::<T>(bare)
        .unwrap_or(default)
}

impl LayoutAlgorithm for BoxLayout {
    fn name(&self) -> &'static str {
        "box"
    }

    fn layout(&self, graph: &mut ElkGraph) -> Result<(), LayoutError> {
        if graph.children.is_empty() {
            return Ok(());
        }

        let aspect_ratio = option(graph, "aspect_ratio", 1.6);
        let spacing = graph.layout_options.node_spacing();

        let count = graph.children.len();
        let cols = (count as f64 * aspect_ratio).sqrt().ceil().max(1.0) as usize;

        let cell_width = graph
            .children
            .iter()
            .map(|n| n.width.unwrap_or(DEFAULT_NODE_WIDTH))
            .fold(0.0, f64::max);
        let cell_height = graph
            .children
            .iter()
            .map(|n| n.height.unwrap_or(DEFAULT_NODE_HEIGHT))
            .fold(0.0, f64::max);

        for (i, node) in graph.children.iter_mut().enumerate() {
            node.x = Some((i % cols) as f64 * (cell_width + spacing));
            node.y = Some((i / cols) as f64 * (cell_height + spacing));
        }

        apply_padding(graph);
        straight_sections(graph)
    }
}
