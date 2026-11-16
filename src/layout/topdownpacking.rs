//! The TopdownPacking algorithm: the port of elkrb's `TopdownPacking`.
//!
//! A uniform grid approximating a square: cols = ceil(sqrt(n)), rows
//! filling without an empty last row. Every node takes the uniform
//! cell size — the `topdownpacking.nodeWidth` option when set, else
//! the nodes' average dimensions shaped by `topdownpacking.aspectRatio`
//! (1.0) — and the grid places them left to right, top to bottom.

use super::{apply_padding, straight_sections, LayoutAlgorithm, LayoutError};
use crate::elk::ElkGraph;
use std::str::FromStr;

pub struct TopdownPacking;

fn option<T: FromStr>(graph: &ElkGraph, key: &str) -> Option<T> {
    graph.layout_options.get_parsed::<T>(key)
}

impl LayoutAlgorithm for TopdownPacking {
    fn name(&self) -> &'static str {
        "topdownpacking"
    }

    fn layout(&self, graph: &mut ElkGraph) -> Result<(), LayoutError> {
        if graph.children.is_empty() {
            return Ok(());
        }
        if graph.children.len() == 1 {
            graph.children[0].x = Some(0.0);
            graph.children[0].y = Some(0.0);
            apply_padding(graph);
            return straight_sections(graph);
        }

        let count = graph.children.len();
        let cols = (count as f64).sqrt().ceil().max(1.0) as usize;

        let ratio = option::<f64>(graph, "topdownpacking.aspectRatio")
            .filter(|r| *r > 0.0)
            .unwrap_or(1.0);
        let (cell_width, cell_height) = match option::<f64>(graph, "topdownpacking.nodeWidth") {
            Some(width) => (width, width / ratio),
            None => {
                let n = count as f64;
                let avg_width = graph
                    .children
                    .iter()
                    .map(|n| n.width.unwrap_or(0.0))
                    .sum::<f64>()
                    / n;
                let avg_height = graph
                    .children
                    .iter()
                    .map(|n| n.height.unwrap_or(0.0))
                    .sum::<f64>()
                    / n;
                if avg_width >= avg_height {
                    (avg_width, avg_width / ratio)
                } else {
                    (avg_height * ratio, avg_height)
                }
            }
        };

        let spacing = graph.layout_options.node_spacing();
        for (i, node) in graph.children.iter_mut().enumerate() {
            node.width = Some(cell_width);
            node.height = Some(cell_height);
            node.x = Some((i % cols) as f64 * (cell_width + spacing));
            node.y = Some((i / cols) as f64 * (cell_height + spacing));
        }

        apply_padding(graph);
        straight_sections(graph)
    }
}
