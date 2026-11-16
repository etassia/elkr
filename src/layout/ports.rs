//! Port constraints: the port of elkrb's `PortConstraintProcessor`.
//!
//! Ports are fixed connection points on a node's boundary. Each port
//! declares a side (NORTH, SOUTH, EAST, WEST, or UNDEFINED) and an
//! order index; the constraint pass detects undefined sides from the
//! port's relative position, orders the ports on each side (explicit
//! index first, then position), and distributes them evenly along the
//! side's edge. Edge endpoints naming a port anchor there; endpoints
//! naming a node keep the algorithm's default anchoring.

use crate::elk::{ElkGraph, ElkNode};

pub const NORTH: &str = "NORTH";
pub const SOUTH: &str = "SOUTH";
pub const EAST: &str = "EAST";
pub const WEST: &str = "WEST";
pub const UNDEFINED: &str = "UNDEFINED";

/// The absolute connection point of a port id that names a port of
/// `node`, or None when the id is not one of its ports.
pub fn port_anchor(node: &ElkNode, port_id: &str) -> Option<(f64, f64)> {
    let port = node.ports.iter().find(|p| p.id == port_id)?;
    Some((
        node.x.unwrap_or(0.0) + port.x,
        node.y.unwrap_or(0.0) + port.y,
    ))
}

/// The node index a graph edge endpoint belongs to: the node itself,
/// or the owner of the named port.
pub fn endpoint_node(
    graph: &ElkGraph,
    index_of: &std::collections::BTreeMap<&str, usize>,
    endpoint: &str,
) -> Option<usize> {
    if let Some(&i) = index_of.get(endpoint) {
        return Some(i);
    }
    graph
        .children
        .iter()
        .position(|n| n.ports.iter().any(|p| p.id == endpoint))
}

/// elkrb's constraint pass over every node that carries ports.
pub fn apply_port_constraints(graph: &mut ElkGraph) {
    for node in &mut graph.children {
        if node.ports.is_empty() {
            continue;
        }
        let width = node.width.unwrap_or(0.0);
        let height = node.height.unwrap_or(0.0);
        if width <= 0.0 || height <= 0.0 {
            continue;
        }
        detect_sides(node, width, height);
        let mut by_side: Vec<Vec<usize>> = Vec::new();
        for side in [NORTH, SOUTH, WEST, EAST] {
            let mut on_side: Vec<usize> = node
                .ports
                .iter()
                .enumerate()
                .filter(|(_, p)| p.side == side)
                .map(|(i, _)| i)
                .collect();
            order_on_side(node, side, &mut on_side);
            if !on_side.is_empty() {
                by_side.push(on_side);
            }
        }
        distribute(node, &by_side);
    }
}

fn detect_sides(node: &mut ElkNode, width: f64, height: f64) {
    for port in &mut node.ports {
        if port.side != UNDEFINED {
            continue;
        }
        // distance from each side, in relative 0..1 coordinates
        let rel_x = port.x / width;
        let rel_y = port.y / height;
        let distances = [
            (NORTH, rel_y),
            (SOUTH, 1.0 - rel_y),
            (WEST, rel_x),
            (EAST, 1.0 - rel_x),
        ];
        let best = distances
            .iter()
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(side, _)| side)
            .copied()
            .unwrap_or(UNDEFINED);
        port.side = best.to_string();
    }
}

fn order_on_side(node: &mut ElkNode, side: &str, order: &mut Vec<usize>) {
    let mut ranks: Vec<(usize, f64)> = order
        .iter()
        .map(|&i| {
            let p = &node.ports[i];
            let rank = if p.index >= 0 {
                p.index as f64
            } else if side == NORTH || side == SOUTH {
                p.x
            } else {
                p.y
            };
            (i, rank)
        })
        .collect();
    ranks.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    *order = ranks.iter().map(|(i, _)| *i).collect();
    for (position, &i) in order.iter().enumerate() {
        if node.ports[i].index < 0 {
            node.ports[i].index = position as i64;
        }
    }
}

fn distribute(node: &mut ElkNode, by_side: &[Vec<usize>]) {
    for order in by_side {
        let (side, count) = (node.ports[order[0]].side.as_str(), order.len());
        match side {
            NORTH | SOUTH => {
                let edge_y = if side == NORTH {
                    0.0
                } else {
                    node.height.unwrap_or(0.0)
                };
                let spacing = node.width.unwrap_or(0.0) / (count + 1) as f64;
                for (idx, &i) in order.iter().enumerate() {
                    let port = &mut node.ports[i];
                    port.x = spacing * (idx + 1) as f64;
                    port.y = edge_y;
                }
            }
            _ => {
                let edge_x = if side == WEST {
                    0.0
                } else {
                    node.width.unwrap_or(0.0)
                };
                let spacing = node.height.unwrap_or(0.0) / (count + 1) as f64;
                for (idx, &i) in order.iter().enumerate() {
                    let port = &mut node.ports[i];
                    port.x = edge_x;
                    port.y = spacing * (idx + 1) as f64;
                }
            }
        }
    }
}
