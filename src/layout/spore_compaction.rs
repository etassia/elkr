//! The SPOrE Compaction algorithm: the port of elkrb's `SporeCompaction`.
//!
//! Removes whitespace while keeping the relative order: nodes sweep
//! left (then up, when `spore.compactionDirection` is `both`, the
//! default) as far as the nearest node they overlap with on the other
//! axis allows, plus `spore.nodeSpacing` (10). Everything then
//! normalizes to the origin and lands on the padding.

use super::{apply_padding, straight_sections, LayoutAlgorithm, LayoutError};
use crate::elk::ElkGraph;

pub struct SporeCompaction;

const DEFAULT_NODE_SPACING: f64 = 10.0;

impl LayoutAlgorithm for SporeCompaction {
    fn name(&self) -> &'static str {
        "spore-compaction"
    }

    fn layout(&self, graph: &mut ElkGraph) -> Result<(), LayoutError> {
        if graph.children.is_empty() {
            return Ok(());
        }

        let direction = graph
            .layout_options
            .get("spore.compactionDirection")
            .map(String::as_str)
            .unwrap_or("both")
            .to_string();
        let spacing = graph
            .layout_options
            .get_parsed::<f64>("spore.nodeSpacing")
            .unwrap_or(DEFAULT_NODE_SPACING);

        match direction.as_str() {
            "horizontal" => self.compact_x(graph, spacing),
            "vertical" => self.compact_y(graph, spacing),
            _ => {
                self.compact_x(graph, spacing);
                self.compact_y(graph, spacing);
            }
        }

        self.normalize(graph);
        apply_padding(graph);
        straight_sections(graph)
    }
}

impl SporeCompaction {
    fn compact_x(&self, graph: &mut ElkGraph, spacing: f64) {
        // sweep by ascending x; ties broken by the input order
        let mut order: Vec<usize> = (0..graph.children.len()).collect();
        order.sort_by(|&a, &b| {
            let xa = graph.children[a].x.unwrap_or(0.0);
            let xb = graph.children[b].x.unwrap_or(0.0);
            xa.partial_cmp(&xb)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.cmp(&b))
        });
        for position in 1..order.len() {
            let target = order[position];
            let (x, y, h) = {
                let n = &graph.children[target];
                (
                    n.x.unwrap_or(0.0),
                    n.y.unwrap_or(0.0),
                    n.height.unwrap_or(0.0),
                )
            };
            // the rightmost edge among already-swept nodes that
            // vertically overlap this one
            let mut boundary = 0.0;
            let mut any = false;
            for &left in &order[..position] {
                let n = &graph.children[left];
                let (lx, ly, lh) = (
                    n.x.unwrap_or(0.0),
                    n.y.unwrap_or(0.0),
                    n.height.unwrap_or(0.0),
                );
                let overlaps = !(y + h <= ly || ly + lh <= y);
                if !overlaps {
                    continue;
                }
                let right = lx + n.width.unwrap_or(0.0);
                if !any || right > boundary {
                    boundary = right;
                    any = true;
                }
            }
            let boundary = if any { boundary + spacing } else { 0.0 };
            let node = &mut graph.children[target];
            if boundary < x {
                node.x = Some(boundary);
            }
        }
    }

    fn compact_y(&self, graph: &mut ElkGraph, spacing: f64) {
        let mut order: Vec<usize> = (0..graph.children.len()).collect();
        order.sort_by(|&a, &b| {
            let ya = graph.children[a].y.unwrap_or(0.0);
            let yb = graph.children[b].y.unwrap_or(0.0);
            ya.partial_cmp(&yb)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.cmp(&b))
        });
        for position in 1..order.len() {
            let target = order[position];
            let (x, y, w) = {
                let n = &graph.children[target];
                (
                    n.x.unwrap_or(0.0),
                    n.y.unwrap_or(0.0),
                    n.width.unwrap_or(0.0),
                )
            };
            let mut boundary = 0.0;
            let mut any = false;
            for &above in &order[..position] {
                let n = &graph.children[above];
                let (ax, ay, aw) = (
                    n.x.unwrap_or(0.0),
                    n.y.unwrap_or(0.0),
                    n.width.unwrap_or(0.0),
                );
                let overlaps = !(x + w <= ax || ax + aw <= x);
                if !overlaps {
                    continue;
                }
                let bottom = ay + n.height.unwrap_or(0.0);
                if !any || bottom > boundary {
                    boundary = bottom;
                    any = true;
                }
            }
            let boundary = if any { boundary + spacing } else { 0.0 };
            let node = &mut graph.children[target];
            if boundary < y {
                node.y = Some(boundary);
            }
        }
    }

    fn normalize(&self, graph: &mut ElkGraph) {
        let min_x = graph
            .children
            .iter()
            .map(|n| n.x.unwrap_or(0.0))
            .fold(f64::INFINITY, f64::min);
        let min_y = graph
            .children
            .iter()
            .map(|n| n.y.unwrap_or(0.0))
            .fold(f64::INFINITY, f64::min);
        for node in &mut graph.children {
            node.x = Some(node.x.unwrap_or(0.0) - min_x);
            node.y = Some(node.y.unwrap_or(0.0) - min_y);
        }
    }
}
