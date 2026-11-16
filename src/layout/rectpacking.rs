//! The RectPacking algorithm: the port of elkrb's `RectPacking`.
//!
//! Shelf-based bin packing: nodes sort tallest-first and fill shelves
//! left to right; a node joins the current shelf while the shelf is at
//! least 80% of its height (keeping shelf heights uniform), and a
//! shorter node would open the next shelf below. With the
//! tallest-first order the shelf height is always the maximum placed
//! so far, so in practice the whole graph shares one shelf — the same
//! holds in elkrb, and this port is faithful to that.

use super::{apply_padding, straight_sections, LayoutAlgorithm, LayoutError};
use crate::elk::ElkGraph;

pub struct RectPacking;

struct Shelf {
    y: f64,
    height: f64,
    width: f64,
}

impl LayoutAlgorithm for RectPacking {
    fn name(&self) -> &'static str {
        "rectpacking"
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

        let spacing = graph.layout_options.node_spacing();

        // tallest first, ties broken by the input order
        let mut order: Vec<usize> = (0..graph.children.len()).collect();
        order.sort_by(|&a, &b| {
            let ha = graph.children[a].height.unwrap_or(0.0);
            let hb = graph.children[b].height.unwrap_or(0.0);
            hb.partial_cmp(&ha)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.cmp(&b))
        });

        let mut shelf = Shelf {
            y: 0.0,
            height: 0.0,
            width: 0.0,
        };
        for (placed, &i) in order.iter().enumerate() {
            let height = graph.children[i].height.unwrap_or(0.0);
            if placed > 0 && shelf.height < height * 0.8 {
                shelf = Shelf {
                    y: shelf.y + shelf.height + spacing,
                    height: 0.0,
                    width: 0.0,
                };
            }
            let node = &mut graph.children[i];
            node.x = Some(shelf.width);
            node.y = Some(shelf.y);
            shelf.width += node.width.unwrap_or(0.0) + spacing;
            shelf.height = shelf.height.max(height);
        }

        apply_padding(graph);
        straight_sections(graph)
    }
}
