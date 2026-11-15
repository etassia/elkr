//! The Layered (Sugiyama) algorithm: the v0.1 port of elkrb's layered
//! pipeline. Four stages, each deterministic:
//!
//! 1. cycle breaking — a DFS sweep reverses back edges for layering only
//!    (sections keep the original source-to-target direction);
//! 2. layer assignment — longest path from the sources, matching
//!    elkrb's `LayerAssigner`;
//! 3. crossing reduction — four barycenter sweeps, ties broken by the
//!    input order, so the same input always yields the same order;
//! 4. coordinate assignment and routing — layers placed sequentially
//!    along the direction axis with elkrb's spacing knobs, nodes stacked
//!    inside the layer, edges routed as straight sections between node
//!    anchors.
//!
//! v0.1 lays out the flat child list of the root; hierarchies, ports,
//! labels, hyperedges and the remaining ELK options are future work and
//! tracked in the README roadmap.

use super::{LayoutAlgorithm, LayoutError};
use crate::elk::{Direction, EdgeSection, ElkGraph, Point};
use std::collections::{BTreeMap, BTreeSet};

pub struct Layered;

const DEFAULT_NODE_WIDTH: f64 = 40.0;
const DEFAULT_NODE_HEIGHT: f64 = 40.0;

#[derive(Debug, Clone)]
struct FlatNode {
    id: String,
    order: usize,
    width: f64,
    height: f64,
}

impl LayoutAlgorithm for Layered {
    fn name(&self) -> &'static str {
        "layered"
    }

    fn layout(&self, graph: &mut ElkGraph) -> Result<(), LayoutError> {
        let direction = graph.layout_options.direction();
        let node_spacing = graph.layout_options.node_spacing();
        let layer_spacing = graph.layout_options.layer_spacing();

        // ---- flatten: the layout surface is the root's child list ----
        let nodes: Vec<FlatNode> = graph
            .children
            .iter()
            .enumerate()
            .map(|(order, n)| FlatNode {
                id: n.id.clone(),
                order,
                width: n.width.unwrap_or(DEFAULT_NODE_WIDTH),
                height: n.height.unwrap_or(DEFAULT_NODE_HEIGHT),
            })
            .collect();
        let node_count = nodes.len();
        let index_of: BTreeMap<&str, usize> =
            nodes.iter().map(|n| (n.id.as_str(), n.order)).collect();

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

        // ---- 1. cycle breaking ----
        // iterative DFS carrying the edge that entered each node; an edge
        // that reaches a node still on the stack is a back edge and is
        // reversed for layering (elkrb's CycleBreaker semantics)
        let mut out_adj: BTreeMap<usize, Vec<(usize, usize)>> = BTreeMap::new();
        for (ei, &(s, t)) in edges.iter().enumerate() {
            out_adj.entry(s).or_default().push((t, ei));
        }
        let mut state = vec![0u8; node_count]; // 0 unseen, 1 on stack, 2 done
        let mut reversed: BTreeSet<usize> = BTreeSet::new();
        for root in 0..node_count {
            if state[root] != 0 {
                continue;
            }
            state[root] = 1;
            // (node, next child slot), plus the path of nodes on the stack
            let mut stack: Vec<(usize, usize)> = vec![(root, 0)];
            let mut path: Vec<usize> = vec![root];
            while let Some(&mut (n, ref mut consumed)) = stack.last_mut() {
                let kids = out_adj.get(&n).cloned().unwrap_or_default();
                if *consumed < kids.len() {
                    let (next, ei) = kids[*consumed];
                    *consumed += 1;
                    match state[next] {
                        0 => {
                            state[next] = 1;
                            path.push(next);
                            stack.push((next, 0));
                        }
                        1 => {
                            reversed.insert(ei);
                        }
                        _ => {}
                    }
                } else {
                    state[n] = 2;
                    stack.pop();
                    path.pop();
                }
            }
        }
        let acyclic: Vec<(usize, usize)> = edges
            .iter()
            .enumerate()
            .map(|(ei, &(s, t))| {
                if reversed.contains(&ei) {
                    (t, s)
                } else {
                    (s, t)
                }
            })
            .collect();

        // ---- 2. layer assignment: longest path from the sources ----
        let mut indegree = vec![0usize; node_count];
        let mut succ: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for &(s, t) in &acyclic {
            succ.entry(s).or_default().push(t);
            indegree[t] += 1;
        }
        let mut layer = vec![0usize; node_count];
        let mut queue: Vec<usize> = (0..node_count).filter(|&n| indegree[n] == 0).collect();
        let mut head = 0;
        while head < queue.len() {
            let n = queue[head];
            head += 1;
            if let Some(kids) = succ.get(&n) {
                for &t in kids {
                    layer[t] = layer[t].max(layer[n] + 1);
                    indegree[t] -= 1;
                    if indegree[t] == 0 {
                        queue.push(t);
                    }
                }
            }
        }

        // ---- 3. crossing reduction: barycenter sweeps ----
        let max_layer = layer.iter().copied().max().unwrap_or(0);
        let mut layers: Vec<Vec<usize>> = vec![Vec::new(); max_layer + 1];
        for n in 0..node_count {
            layers[layer[n]].push(n);
        }
        let mut preds: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        let mut succs: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for &(s, t) in &acyclic {
            succs.entry(s).or_default().push(t);
            preds.entry(t).or_default().push(s);
        }
        for _ in 0..2 {
            for li in 1..layers.len() {
                let pos: BTreeMap<usize, usize> = layers[li - 1]
                    .iter()
                    .enumerate()
                    .map(|(i, &n)| (n, i))
                    .collect();
                layers[li].sort_by(|&a, &b| {
                    barycenter(a, &preds, &pos)
                        .partial_cmp(&barycenter(b, &preds, &pos))
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then(nodes[a].order.cmp(&nodes[b].order))
                });
            }
            for li in (0..layers.len().saturating_sub(1)).rev() {
                let pos: BTreeMap<usize, usize> = layers[li + 1]
                    .iter()
                    .enumerate()
                    .map(|(i, &n)| (n, i))
                    .collect();
                layers[li].sort_by(|&a, &b| {
                    barycenter(a, &succs, &pos)
                        .partial_cmp(&barycenter(b, &succs, &pos))
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then(nodes[a].order.cmp(&nodes[b].order))
                });
            }
        }

        // ---- 4. coordinates ----
        // `along` is the direction axis, `across` the stacking axis
        let horizontal = matches!(direction, Direction::Right | Direction::Left);
        let layer_extent = |n: &FlatNode| if horizontal { n.width } else { n.height };
        let cross_extent = |n: &FlatNode| if horizontal { n.height } else { n.width };

        let mut along_of = vec![0.0f64; node_count];
        let mut across_of = vec![0.0f64; node_count];
        let mut along_cursor = 0.0;
        for order in &layers {
            let layer_max = order
                .iter()
                .map(|&n| layer_extent(&nodes[n]))
                .fold(0.0f64, f64::max);
            let mut across_cursor = 0.0;
            for &n in order {
                along_of[n] = along_cursor;
                across_of[n] = across_cursor;
                across_cursor += cross_extent(&nodes[n]) + node_spacing;
            }
            along_cursor += layer_max + layer_spacing;
        }
        let along_total = layers
            .last()
            .and_then(|last| last.first().map(|&n| along_of[n] + layer_extent(&nodes[n])))
            .unwrap_or(0.0);

        // write node geometry: Right/Down are the base orientations;
        // Left/Up mirror the along axis (ELK computes one direction and
        // mirrors)
        let mirror = matches!(direction, Direction::Left | Direction::Up);
        for (i, node) in graph.children.iter_mut().enumerate() {
            let along = if mirror {
                along_total - along_of[i] - layer_extent(&nodes[i])
            } else {
                along_of[i]
            };
            let across = across_of[i];
            let (x, y) = if horizontal {
                (along, across)
            } else {
                (across, along)
            };
            node.x = Some(x);
            node.y = Some(y);
        }

        // ---- edge sections: straight, between direction anchors ----
        for e in graph.edges.iter_mut() {
            let (Some(src), Some(tgt)) = (e.sources.first(), e.targets.first()) else {
                continue;
            };
            if src == tgt {
                continue;
            }
            let si = index_of[src.as_str()];
            let ti = index_of[tgt.as_str()];
            let start = anchor(
                &nodes[si],
                along_of[si],
                across_of[si],
                horizontal,
                mirror,
                false,
            );
            let end = anchor(
                &nodes[ti],
                along_of[ti],
                across_of[ti],
                horizontal,
                mirror,
                true,
            );
            e.sections.push(EdgeSection {
                id: format!("{}_section", e.id),
                start_point: start,
                end_point: end,
                bend_points: Vec::new(),
                incoming_shape: Some(nodes[si].id.clone()),
                outgoing_shape: Some(nodes[ti].id.clone()),
            });
        }

        Ok(())
    }
}

fn barycenter(
    n: usize,
    neighbors: &BTreeMap<usize, Vec<usize>>,
    pos: &BTreeMap<usize, usize>,
) -> f64 {
    match neighbors.get(&n) {
        Some(list) if !list.is_empty() => {
            let known: Vec<usize> = list.iter().filter_map(|m| pos.get(m).copied()).collect();
            if known.is_empty() {
                f64::INFINITY
            } else {
                let sum: usize = known.iter().sum();
                sum as f64 / known.len() as f64
            }
        }
        _ => f64::INFINITY,
    }
}

/// The connection point on a node's direction-facing side. Sources offer
/// their outgoing side; targets their incoming side.
fn anchor(
    n: &FlatNode,
    along: f64,
    across: f64,
    horizontal: bool,
    mirror: bool,
    is_target: bool,
) -> Point {
    let (x, y) = if horizontal {
        (along, across)
    } else {
        (across, along)
    };
    // in the mirrored orientations the node rectangle still starts at its
    // stored x/y, so the along-facing side flips with `mirror`: the
    // source offers its far side under Right/Down and its near side
    // under Left/Up
    let on_far_side = is_target == mirror;
    if horizontal {
        Point {
            x: if on_far_side { x + n.width } else { x },
            y: y + n.height / 2.0,
        }
    } else {
        Point {
            x: x + n.width / 2.0,
            y: if on_far_side { y + n.height } else { y },
        }
    }
}
