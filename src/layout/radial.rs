//! The Radial algorithm: the port of elkrb's `Radial`.
//!
//! Places the nodes on one circle, angles evenly stepped from the
//! input order. The radius fits the whole circumference on the nodes'
//! average size (times 1.2 of slack, never under 100), and each node
//! is centered on its point of the circle.

use super::{apply_padding, straight_sections, LayoutAlgorithm, LayoutError};
use crate::elk::ElkGraph;

pub struct Radial;

impl LayoutAlgorithm for Radial {
    fn name(&self) -> &'static str {
        "radial"
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

        let count = graph.children.len() as f64;
        let avg_width = graph
            .children
            .iter()
            .map(|n| n.width.unwrap_or(0.0))
            .sum::<f64>()
            / count;
        let avg_height = graph
            .children
            .iter()
            .map(|n| n.height.unwrap_or(0.0))
            .sum::<f64>()
            / count;
        let avg_size = (avg_width + avg_height) / 2.0;
        let radius = ((count * avg_size) / (2.0 * std::f64::consts::PI) * 1.2).max(100.0);

        let angle_step = 2.0 * std::f64::consts::PI / count;
        for (index, node) in graph.children.iter_mut().enumerate() {
            let angle = index as f64 * angle_step;
            node.x = Some(radius + radius * angle.cos() - node.width.unwrap_or(0.0) / 2.0);
            node.y = Some(radius + radius * angle.sin() - node.height.unwrap_or(0.0) / 2.0);
        }

        apply_padding(graph);
        straight_sections(graph)
    }
}
