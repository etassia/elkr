//! The SPOrE Overlap algorithm: the port of elkrb's `SporeOverlap`.
//!
//! Iterative overlap removal that preserves the layout's structure:
//! bounding boxes inflated by half the spacing in every direction,
//! and every overlapping pair pushed apart by half the overlap along
//! their dominant axis, until no pair overlaps or `spore.maxIterations`
//! (50) rounds pass. The minimum gap is `spore.nodeSpacing` (10).
//! elkrb reads both options from its option map without converting,
//! where the string values would fail its own arithmetic — elkr parses
//! them, so the knobs actually work.

use super::{apply_padding, straight_sections, LayoutAlgorithm, LayoutError};
use crate::elk::ElkGraph;

pub struct SporeOverlap;

const DEFAULT_MAX_ITERATIONS: usize = 50;
const DEFAULT_NODE_SPACING: f64 = 10.0;

impl LayoutAlgorithm for SporeOverlap {
    fn name(&self) -> &'static str {
        "spore-overlap"
    }

    fn layout(&self, graph: &mut ElkGraph) -> Result<(), LayoutError> {
        if graph.children.is_empty() {
            return Ok(());
        }

        let max_iterations = graph
            .layout_options
            .get_parsed::<usize>("spore.maxIterations")
            .unwrap_or(DEFAULT_MAX_ITERATIONS);
        let spacing = graph
            .layout_options
            .get_parsed::<f64>("spore.nodeSpacing")
            .unwrap_or(DEFAULT_NODE_SPACING);

        for _ in 0..max_iterations {
            let overlaps = self.overlaps(graph, spacing);
            if overlaps.is_empty() {
                break;
            }
            self.resolve(graph, &overlaps, spacing);
        }

        apply_padding(graph);
        straight_sections(graph)
    }
}

impl SporeOverlap {
    /// Overlapping pairs as index pairs, in input order.
    fn overlaps(&self, graph: &ElkGraph, spacing: f64) -> Vec<(usize, usize)> {
        let mut pairs = Vec::new();
        for i in 0..graph.children.len() {
            for j in (i + 1)..graph.children.len() {
                if self.overlapping(graph, i, j, spacing) {
                    pairs.push((i, j));
                }
            }
        }
        pairs
    }

    fn overlapping(&self, graph: &ElkGraph, i: usize, j: usize, spacing: f64) -> bool {
        let half = spacing / 2.0;
        let a = &graph.children[i];
        let b = &graph.children[j];
        let (ax, ay) = (a.x.unwrap_or(0.0), a.y.unwrap_or(0.0));
        let (bx, by) = (b.x.unwrap_or(0.0), b.y.unwrap_or(0.0));
        let left1 = ax - half;
        let right1 = ax + a.width.unwrap_or(0.0) + half;
        let top1 = ay - half;
        let bottom1 = ay + a.height.unwrap_or(0.0) + half;
        let left2 = bx - half;
        let right2 = bx + b.width.unwrap_or(0.0) + half;
        let top2 = by - half;
        let bottom2 = by + b.height.unwrap_or(0.0) + half;
        !(right1 <= left2 || left1 >= right2 || bottom1 <= top2 || top1 >= bottom2)
    }

    fn resolve(&self, graph: &mut ElkGraph, pairs: &[(usize, usize)], spacing: f64) {
        for &(i, j) in pairs {
            let (w1, h1) = {
                let n = &graph.children[i];
                (n.width.unwrap_or(0.0), n.height.unwrap_or(0.0))
            };
            let (w2, h2) = {
                let n = &graph.children[j];
                (n.width.unwrap_or(0.0), n.height.unwrap_or(0.0))
            };
            let (c1x, c1y) = {
                let n = &graph.children[i];
                (n.x.unwrap_or(0.0) + w1 / 2.0, n.y.unwrap_or(0.0) + h1 / 2.0)
            };
            let (c2x, c2y) = {
                let n = &graph.children[j];
                (n.x.unwrap_or(0.0) + w2 / 2.0, n.y.unwrap_or(0.0) + h2 / 2.0)
            };
            let dx = c2x - c1x;
            let dy = c2y - c1y;
            let min_dist_x = (w1 + w2) / 2.0 + spacing;
            let min_dist_y = (h1 + h2) / 2.0 + spacing;

            // elkrb's movement: push the pair apart by half the overlap
            // along the dominant axis, node2 away from node1
            let (shift_x, shift_y) = if dx.abs() > dy.abs() {
                let overlap = if dx > 0.0 {
                    min_dist_x - dx
                } else {
                    min_dist_x + dx
                };
                if overlap > 0.0 {
                    (overlap / 2.0 * dx.signum(), 0.0)
                } else {
                    (0.0, 0.0)
                }
            } else {
                let overlap = if dy > 0.0 {
                    min_dist_y - dy
                } else {
                    min_dist_y + dy
                };
                if overlap > 0.0 {
                    (0.0, overlap / 2.0 * dy.signum())
                } else {
                    (0.0, 0.0)
                }
            };

            let a = &mut graph.children[i];
            a.x = Some(a.x.unwrap_or(0.0) - shift_x);
            a.y = Some(a.y.unwrap_or(0.0) - shift_y);
            let b = &mut graph.children[j];
            b.x = Some(b.x.unwrap_or(0.0) + shift_x);
            b.y = Some(b.y.unwrap_or(0.0) + shift_y);
        }
    }
}
