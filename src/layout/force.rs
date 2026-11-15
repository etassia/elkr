//! The Force-directed algorithm: the port of elkrb's `Force`.
//!
//! Nodes repel each other in inverse proportion to the square of their
//! distance; edges act as springs pulling connected nodes together. The
//! simulation cools linearly from the temperature cap to zero over
//! `iterations` steps, moving every node by at most the current cap
//! along its net force each step.
//!
//! elkrb seeds the initial scatter from `Kernel#rand`, which cannot
//! satisfy this crate's byte-identity contract: the same input must lay
//! out to the same bytes on every run. elkr draws the scatter from a
//! fixed-seed SplitMix64 stream instead, and preserves node coordinates
//! already present on the input as the start, exactly as elkrb does.
//!
//! Options (bare key first, then the ELK-style alias; elkrb reads only
//! the bare key): `iterations` / `elk.force.iterations` (300),
//! `repulsion` / `elk.force.repulsion` (5.0), `temperature` /
//! `elk.force.temperature` (0.001).

use super::{LayoutAlgorithm, LayoutError};
use crate::elk::{EdgeSection, ElkGraph, Point};
use std::collections::BTreeMap;
use std::str::FromStr;

pub struct Force;

const DEFAULT_ITERATIONS: usize = 300;
const DEFAULT_REPULSION: f64 = 5.0;
const DEFAULT_TEMPERATURE: f64 = 0.001;
const PADDING: f64 = 12.0;
const DEFAULT_NODE_WIDTH: f64 = 40.0;
const DEFAULT_NODE_HEIGHT: f64 = 40.0;
const SCATTER_SEED: u64 = 0x2545_F491_4F6C_DD1D;

/// SplitMix64: a fixed-seed stream stands in for elkrb's `Kernel#rand`,
/// so the initial scatter is identical on every run and every platform.
struct Scatter(u64);

impl Scatter {
    fn next_unit(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        ((z >> 11) as f64) / (1u64 << 53) as f64
    }
}

fn option<T: FromStr>(graph: &ElkGraph, bare: &str, elk: &str) -> Option<T> {
    graph
        .layout_options
        .get_parsed::<T>(bare)
        .or_else(|| graph.layout_options.get_parsed::<T>(elk))
}

impl LayoutAlgorithm for Force {
    fn name(&self) -> &'static str {
        "force"
    }

    fn layout(&self, graph: &mut ElkGraph) -> Result<(), LayoutError> {
        if graph.children.is_empty() {
            return Ok(());
        }

        let iterations = option::<usize>(graph, "iterations", "elk.force.iterations")
            .unwrap_or(DEFAULT_ITERATIONS);
        let repulsion =
            option::<f64>(graph, "repulsion", "elk.force.repulsion").unwrap_or(DEFAULT_REPULSION);
        let temperature = option::<f64>(graph, "temperature", "elk.force.temperature")
            .unwrap_or(DEFAULT_TEMPERATURE);

        let count = graph.children.len();
        let index_of: BTreeMap<String, usize> = graph
            .children
            .iter()
            .enumerate()
            .map(|(i, n)| (n.id.clone(), i))
            .collect();

        // edges with both endpoints on the flat surface; self loops skip
        let mut edges: Vec<(usize, usize)> = Vec::new();
        for e in &graph.edges {
            let (Some(src), Some(tgt)) = (e.sources.first(), e.targets.first()) else {
                continue;
            };
            if src == tgt {
                continue;
            }
            let si = *index_of
                .get(src.as_str())
                .ok_or_else(|| LayoutError::UnknownNode {
                    edge: e.id.clone(),
                    node: src.clone(),
                })?;
            let ti = *index_of
                .get(tgt.as_str())
                .ok_or_else(|| LayoutError::UnknownNode {
                    edge: e.id.clone(),
                    node: tgt.clone(),
                })?;
            edges.push((si, ti));
        }

        // ---- initial scatter ----
        // elkrb: nodes without a complete position start inside a square
        // whose side is the square root of the padded node areas
        let mut pos: Vec<(f64, f64)> = Vec::with_capacity(count);
        let side: f64 = graph
            .children
            .iter()
            .map(|n| {
                (n.width.unwrap_or(DEFAULT_NODE_WIDTH) + 20.0)
                    * (n.height.unwrap_or(DEFAULT_NODE_HEIGHT) + 20.0)
            })
            .sum::<f64>()
            .sqrt();
        let mut scatter = Scatter(SCATTER_SEED);
        for node in &graph.children {
            match (node.x, node.y) {
                (Some(x), Some(y)) => pos.push((x, y)),
                _ => pos.push((scatter.next_unit() * side, scatter.next_unit() * side)),
            }
        }

        // ---- simulation ----
        let mut forces = vec![(0.0f64, 0.0f64); count];
        for step in 0..iterations {
            let temp = temperature * (1.0 - (step as f64) / (iterations as f64));

            forces.fill((0.0, 0.0));

            // repulsion between every pair
            for a in 0..count {
                for b in (a + 1)..count {
                    let dx = pos[b].0 - pos[a].0;
                    let dy = pos[b].1 - pos[a].1;
                    let distance_sq = dx * dx + dy * dy;
                    if distance_sq < 0.01 {
                        continue;
                    }
                    let strength = repulsion / distance_sq;
                    let inv = 1.0 / distance_sq.sqrt();
                    forces[a].0 -= dx * inv * strength;
                    forces[a].1 -= dy * inv * strength;
                    forces[b].0 += dx * inv * strength;
                    forces[b].1 += dy * inv * strength;
                }
            }

            // springs along the edges
            for &(s, t) in &edges {
                let dx = pos[t].0 - pos[s].0;
                let dy = pos[t].1 - pos[s].1;
                let distance = (dx * dx + dy * dy).sqrt();
                if distance == 0.0 {
                    continue;
                }
                let strength = distance / 10.0;
                forces[s].0 += dx / distance * strength;
                forces[s].1 += dy / distance * strength;
                forces[t].0 -= dx / distance * strength;
                forces[t].1 -= dy / distance * strength;
            }

            // move each node by at most `temp` along its net force
            for i in 0..count {
                let (fx, fy) = forces[i];
                let magnitude = (fx * fx + fy * fy).sqrt();
                if magnitude == 0.0 {
                    continue;
                }
                let displacement = magnitude.min(temp);
                pos[i].0 += fx / magnitude * displacement;
                pos[i].1 += fy / magnitude * displacement;
            }
        }

        // ---- padding: shift so the bounding box starts at (PAD, PAD) ----
        let min_x = pos.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
        let min_y = pos.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
        for (node, p) in graph.children.iter_mut().zip(&mut pos) {
            node.x = Some(p.0 - min_x + PADDING);
            node.y = Some(p.1 - min_y + PADDING);
        }

        // ---- edge sections: straight, center to center (elkrb's
        // EdgeRouter#route_node_to_node default) ----
        for e in graph.edges.iter_mut() {
            let (Some(src), Some(tgt)) = (e.sources.first(), e.targets.first()) else {
                continue;
            };
            if src == tgt {
                continue;
            }
            let sn = &graph.children[index_of[src.as_str()]];
            let tn = &graph.children[index_of[tgt.as_str()]];
            let center = |n: &crate::elk::ElkNode| Point {
                x: n.x.unwrap_or(0.0) + n.width.unwrap_or(DEFAULT_NODE_WIDTH) / 2.0,
                y: n.y.unwrap_or(0.0) + n.height.unwrap_or(DEFAULT_NODE_HEIGHT) / 2.0,
            };
            e.sections.push(EdgeSection {
                id: format!("{}_section", e.id),
                start_point: center(sn),
                end_point: center(tn),
                bend_points: Vec::new(),
                incoming_shape: Some(sn.id.clone()),
                outgoing_shape: Some(tn.id.clone()),
            });
        }

        Ok(())
    }
}
