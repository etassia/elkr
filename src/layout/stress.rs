//! The Stress algorithm: the port of elkrb's `Stress`.
//!
//! Stress majorization over shortest-path ideal distances: the graph's
//! hop-count matrix (Floyd-Warshall, edges weighted 1.0) is the ideal,
//! and each iteration moves every node toward the weighted barycenter
//! of where its distances say the others should be. The loop stops
//! when the stress stops improving by `epsilon` (0.0001) or after
//! `iterations` (500). The start is elkrb's circle of radius n * 10.
//! Pairs with no connecting path carry an infinite ideal and pull on
//! nothing; edges with off-surface endpoints skip.

use super::{apply_padding, straight_sections, LayoutAlgorithm, LayoutError};
use crate::elk::ElkGraph;
use std::collections::BTreeMap;
use std::str::FromStr;

pub struct Stress;

const DEFAULT_ITERATIONS: usize = 500;
const DEFAULT_EPSILON: f64 = 0.0001;

fn option<T: FromStr>(graph: &ElkGraph, bare: &str, default: T) -> T {
    graph
        .layout_options
        .get_parsed::<T>(bare)
        .unwrap_or(default)
}

impl LayoutAlgorithm for Stress {
    fn name(&self) -> &'static str {
        "stress"
    }

    fn layout(&self, graph: &mut ElkGraph) -> Result<(), LayoutError> {
        if graph.children.is_empty() {
            return Ok(());
        }

        let iterations = option(graph, "iterations", DEFAULT_ITERATIONS);
        let epsilon = option(graph, "epsilon", DEFAULT_EPSILON);

        let count = graph.children.len();
        let index_of: BTreeMap<String, usize> = graph
            .children
            .iter()
            .enumerate()
            .map(|(i, n)| (n.id.clone(), i))
            .collect();

        // the circle elkrb starts from
        let radius = count as f64 * 10.0;
        for (i, node) in graph.children.iter_mut().enumerate() {
            let angle = 2.0 * std::f64::consts::PI * i as f64 / count as f64;
            node.x = Some(radius * angle.cos() + radius);
            node.y = Some(radius * angle.sin() + radius);
        }

        let ideal = self.hop_distances(graph, &index_of);

        let mut positions: Vec<(f64, f64)> = graph
            .children
            .iter()
            .map(|n| (n.x.unwrap_or(0.0), n.y.unwrap_or(0.0)))
            .collect();
        let mut previous = self.stress(&positions, &ideal);
        for _ in 0..iterations {
            self.optimize(&mut positions, &ideal);
            let current = self.stress(&positions, &ideal);
            if (previous - current).abs() < epsilon {
                break;
            }
            previous = current;
        }

        for (node, p) in graph.children.iter_mut().zip(&positions) {
            node.x = Some(p.0);
            node.y = Some(p.1);
        }

        apply_padding(graph);
        straight_sections(graph)
    }
}

impl Stress {
    fn hop_distances(&self, graph: &ElkGraph, index_of: &BTreeMap<String, usize>) -> Vec<Vec<f64>> {
        let count = graph.children.len();
        let inf = f64::INFINITY;
        let mut distances = vec![vec![inf; count]; count];
        for (i, row) in distances.iter_mut().enumerate() {
            row[i] = 0.0;
        }
        for e in &graph.edges {
            let (Some(src), Some(tgt)) = (e.sources.first(), e.targets.first()) else {
                continue;
            };
            if src == tgt {
                continue;
            }
            let (Some(&si), Some(&ti)) = (index_of.get(src.as_str()), index_of.get(tgt.as_str()))
            else {
                continue;
            };
            distances[si][ti] = 1.0;
            distances[ti][si] = 1.0;
        }
        for k in 0..count {
            for i in 0..count {
                for j in 0..count {
                    let through = distances[i][k] + distances[k][j];
                    if through < distances[i][j] {
                        distances[i][j] = through;
                    }
                }
            }
        }
        distances
    }

    fn stress(&self, positions: &[(f64, f64)], ideal: &[Vec<f64>]) -> f64 {
        let count = positions.len();
        let mut total = 0.0;
        for i in 0..count {
            for j in (i + 1)..count {
                if ideal[i][j] == f64::INFINITY {
                    continue;
                }
                let dx = positions[j].0 - positions[i].0;
                let dy = positions[j].1 - positions[i].1;
                let actual = (dx * dx + dy * dy).sqrt();
                let diff = actual - ideal[i][j];
                total += diff * diff;
            }
        }
        total
    }

    fn optimize(&self, positions: &mut [(f64, f64)], ideal: &[Vec<f64>]) {
        let count = positions.len();
        for i in 0..count {
            let mut sum_x = 0.0;
            let mut sum_y = 0.0;
            let mut sum_weight = 0.0;
            for j in 0..count {
                if i == j || ideal[i][j] == f64::INFINITY {
                    continue;
                }
                let dx = positions[j].0 - positions[i].0;
                let dy = positions[j].1 - positions[i].1;
                let actual = (dx * dx + dy * dy).sqrt();
                if actual == 0.0 {
                    continue;
                }
                let ideal_dist = ideal[i][j];
                let weight = 1.0 / (ideal_dist * ideal_dist);
                let ratio = ideal_dist / actual;
                sum_x += weight * (positions[j].0 - ratio * dx);
                sum_y += weight * (positions[j].1 - ratio * dy);
                sum_weight += weight;
            }
            if sum_weight > 0.0 {
                positions[i].0 = sum_x / sum_weight;
                positions[i].1 = sum_y / sum_weight;
            }
        }
    }
}
