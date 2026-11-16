//! The Random algorithm: the port of elkrb's `Random`.
//!
//! Places nodes at random positions inside a bounded area whose size
//! comes from the padded node areas and the `aspect_ratio` option
//! (1.6 by default). elkrb draws the positions from `Kernel#rand`,
//! which cannot satisfy this crate's byte-identity contract; the
//! scatter comes from the shared fixed-seed SplitMix64 stream instead,
//! so the same input always lays out to the same bytes.

use super::{apply_padding, straight_sections, LayoutAlgorithm, LayoutError, Scatter};
use crate::elk::ElkGraph;
use std::str::FromStr;

pub struct Random;

const SCATTER_SEED: u64 = 0x1A2B_3C4D_5E6F_8090;

fn option<T: FromStr>(graph: &ElkGraph, bare: &str, default: T) -> T {
    graph
        .layout_options
        .get_parsed::<T>(bare)
        .unwrap_or(default)
}

impl LayoutAlgorithm for Random {
    fn name(&self) -> &'static str {
        "random"
    }

    fn layout(&self, graph: &mut ElkGraph) -> Result<(), LayoutError> {
        if graph.children.is_empty() {
            return Ok(());
        }

        let aspect_ratio = option(graph, "aspect_ratio", 1.6);
        let spacing = graph.layout_options.node_spacing();

        let total_area: f64 = graph
            .children
            .iter()
            .map(|n| {
                let width = n.width.unwrap_or(0.0);
                let height = n.height.unwrap_or(0.0);
                (width + spacing) * (height + spacing)
            })
            .sum();
        let width = (total_area * aspect_ratio).sqrt();
        let height = width / aspect_ratio;

        let mut scatter = Scatter(SCATTER_SEED);
        for node in &mut graph.children {
            let node_width = node.width.unwrap_or(0.0);
            let node_height = node.height.unwrap_or(0.0);
            node.x = Some(scatter.next_unit() * (width - node_width));
            node.y = Some(scatter.next_unit() * (height - node_height));
        }

        apply_padding(graph);
        straight_sections(graph)
    }
}
