//! The Fixed algorithm: the port of elkrb's `Fixed`.
//!
//! Keeps the nodes where the input put them: coordinates already on
//! the graph stay, absent coordinates become the origin, and the
//! result is shifted so the bounding box starts at the padding.

use super::{apply_padding, straight_sections, LayoutAlgorithm, LayoutError};
use crate::elk::ElkGraph;

pub struct Fixed;

impl LayoutAlgorithm for Fixed {
    fn name(&self) -> &'static str {
        "fixed"
    }

    fn layout(&self, graph: &mut ElkGraph) -> Result<(), LayoutError> {
        if graph.children.is_empty() {
            return Ok(());
        }

        for node in &mut graph.children {
            if node.x.is_none() {
                node.x = Some(0.0);
            }
            if node.y.is_none() {
                node.y = Some(0.0);
            }
        }

        apply_padding(graph);
        straight_sections(graph)
    }
}
