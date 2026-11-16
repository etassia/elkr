//! The Libavoid algorithm: the port of elkrb's `Libavoid`.
//!
//! Orthogonal connector routing around obstacles with A*: the
//! obstacles are the nodes inflated by `libavoid.routingPadding` (10),
//! the search walks a grid stepped by the same padding with a
//! Manhattan heuristic, and `libavoid.segmentPenalty` (1.0) plus
//! `libavoid.bendPenalty` (2.0) shape the cost. The result passes
//! through a visibility simplification that drops redundant bends.
//! Nodes without positions land on elkrb's box grid first. One
//! behavior inherited from elkrb unchanged: the path starts at the
//! source's center, inside that node's own inflated obstacle, so
//! every first step collides and the search falls back to the direct
//! path — the routing is straight sections until endpoints start on
//! the obstacle boundary on both sides.

use super::{apply_padding, LayoutAlgorithm, LayoutError};
use crate::elk::{ElkGraph, Point};
use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

pub struct Libavoid;

const DEFAULT_ROUTING_PADDING: f64 = 10.0;
const DEFAULT_SEGMENT_PENALTY: f64 = 1.0;
const DEFAULT_BEND_PENALTY: f64 = 2.0;

#[derive(Clone, Copy)]
struct Rect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Clone, Copy, PartialEq)]
enum Direction {
    Horizontal,
    Vertical,
}

struct PathNode {
    point: Point,
    parent: Option<usize>,
    g_score: f64,
    f_score: f64,
    direction: Option<Direction>,
}

fn option<T: FromStr>(graph: &ElkGraph, key: &str, default: T) -> T {
    graph.layout_options.get_parsed::<T>(key).unwrap_or(default)
}

impl LayoutAlgorithm for Libavoid {
    fn name(&self) -> &'static str {
        "libavoid"
    }

    fn layout(&self, graph: &mut ElkGraph) -> Result<(), LayoutError> {
        if graph.children.is_empty() {
            return Ok(());
        }

        self.position_if_needed(graph);
        apply_padding(graph);

        let padding = option(graph, "libavoid.routingPadding", DEFAULT_ROUTING_PADDING);
        let obstacles: Vec<Rect> = graph
            .children
            .iter()
            .map(|n| Rect {
                x: n.x.unwrap_or(0.0) - padding,
                y: n.y.unwrap_or(0.0) - padding,
                width: n.width.unwrap_or(0.0) + 2.0 * padding,
                height: n.height.unwrap_or(0.0) + 2.0 * padding,
            })
            .collect();

        let index_of: BTreeMap<String, usize> = graph
            .children
            .iter()
            .enumerate()
            .map(|(i, n)| (n.id.clone(), i))
            .collect();

        let section_penalty = option(graph, "libavoid.segmentPenalty", DEFAULT_SEGMENT_PENALTY);
        let bend_penalty = option(graph, "libavoid.bendPenalty", DEFAULT_BEND_PENALTY);

        for e in graph.edges.iter_mut() {
            let (Some(src), Some(tgt)) = (e.sources.first(), e.targets.first()) else {
                continue;
            };
            let (Some(&si), Some(&ti)) = (index_of.get(src.as_str()), index_of.get(tgt.as_str()))
            else {
                continue;
            };
            let start = center(&graph.children[si]);
            let goal = center(&graph.children[ti]);
            let path = self.find_path(
                start,
                goal,
                &obstacles,
                section_penalty,
                bend_penalty,
                padding,
            );
            let bends = simplify(&path, &obstacles, start, goal);
            e.sections.clear();
            e.sections.push(crate::elk::EdgeSection {
                id: format!("{}_section", e.id),
                start_point: start,
                end_point: goal,
                bend_points: bends,
                incoming_shape: Some(src.clone()),
                outgoing_shape: Some(tgt.clone()),
            });
        }

        Ok(())
    }
}

impl Libavoid {
    fn position_if_needed(&self, graph: &mut ElkGraph) {
        if graph
            .children
            .iter()
            .all(|n| n.x.is_some() && n.y.is_some())
        {
            return;
        }
        let spacing = graph.layout_options.node_spacing();
        let cell_width = graph
            .children
            .iter()
            .map(|n| n.width.unwrap_or(0.0))
            .fold(0.0, f64::max);
        let cell_height = graph
            .children
            .iter()
            .map(|n| n.height.unwrap_or(0.0))
            .fold(0.0, f64::max);
        let cols = ((graph.children.len() as f64) * 1.6).sqrt().ceil().max(1.0) as usize;
        for (i, node) in graph.children.iter_mut().enumerate() {
            node.x = Some((i % cols) as f64 * (cell_width + spacing));
            node.y = Some((i / cols) as f64 * (cell_height + spacing));
        }
    }

    fn find_path(
        &self,
        start: Point,
        goal: Point,
        obstacles: &[Rect],
        segment_penalty: f64,
        bend_penalty: f64,
        step: f64,
    ) -> Vec<Point> {
        let mut arena: Vec<PathNode> = vec![PathNode {
            point: start,
            parent: None,
            g_score: 0.0,
            f_score: manhattan(start, goal),
            direction: None,
        }];
        let mut open: Vec<usize> = vec![0];
        let mut closed: BTreeSet<String> = BTreeSet::new();
        let mut g_scores: BTreeMap<String, f64> = BTreeMap::new();
        g_scores.insert(point_key(start), 0.0);

        while !open.is_empty() {
            let current_index = open
                .iter()
                .copied()
                .min_by(|a, b| {
                    let fa = arena[*a].f_score;
                    let fb = arena[*b].f_score;
                    fa.partial_cmp(&fb).unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap();
            let current = &arena[current_index];
            let (current_point, current_g, current_direction) =
                (current.point, current.g_score, current.direction);
            if near(current_point, goal) {
                return Self::reconstruct(&arena, current_index);
            }
            open.retain(|&i| i != current_index);
            closed.insert(point_key(current_point));

            for (neighbor, direction) in Self::neighbors(current_point, step, obstacles) {
                let key = point_key(neighbor);
                if closed.contains(&key) {
                    continue;
                }
                let distance = euclidean(current_point, neighbor);
                let turn = match (current_direction, direction) {
                    (Some(from), to) if from != to => bend_penalty,
                    _ => 0.0,
                };
                let tentative = current_g + distance + segment_penalty + turn;
                let better = g_scores.get(&key).is_none_or(|g| tentative < *g);
                if better {
                    g_scores.insert(key, tentative);
                    arena.push(PathNode {
                        point: neighbor,
                        parent: Some(current_index),
                        g_score: tentative,
                        f_score: tentative + manhattan(neighbor, goal),
                        direction: Some(direction),
                    });
                    open.push(arena.len() - 1);
                }
            }
        }
        vec![start, goal]
    }

    fn neighbors(point: Point, step: f64, obstacles: &[Rect]) -> Vec<(Point, Direction)> {
        let candidates = [
            (step, 0.0, Direction::Horizontal),
            (-step, 0.0, Direction::Horizontal),
            (0.0, step, Direction::Vertical),
            (0.0, -step, Direction::Vertical),
        ];
        candidates
            .into_iter()
            .map(|(dx, dy, direction)| {
                (
                    Point {
                        x: point.x + dx,
                        y: point.y + dy,
                    },
                    direction,
                )
            })
            .filter(|(neighbor, _)| {
                !obstacles
                    .iter()
                    .any(|rect| segment_hits(point, *neighbor, *rect))
            })
            .collect()
    }

    fn reconstruct(arena: &[PathNode], mut index: usize) -> Vec<Point> {
        let mut path = Vec::new();
        while let Some(node) = arena.get(index) {
            path.push(node.point);
            match node.parent {
                Some(parent) => index = parent,
                None => break,
            }
        }
        path.reverse();
        path
    }
}

fn center(n: &crate::elk::ElkNode) -> Point {
    Point {
        x: n.x.unwrap_or(0.0) + n.width.unwrap_or(0.0) / 2.0,
        y: n.y.unwrap_or(0.0) + n.height.unwrap_or(0.0) / 2.0,
    }
}

fn manhattan(a: Point, b: Point) -> f64 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}

fn euclidean(a: Point, b: Point) -> f64 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}

fn near(a: Point, b: Point) -> bool {
    (a.x - b.x).abs() < 0.1 && (a.y - b.y).abs() < 0.1
}

fn point_key(p: Point) -> String {
    format!("{:.2},{:.2}", p.x, p.y)
}

/// elkrb's `line_intersects_rectangle?`: either endpoint inside the
/// rectangle, or a proper crossing with one of its edges.
fn segment_hits(p1: Point, p2: Point, rect: Rect) -> bool {
    if point_in(p1, rect) || point_in(p2, rect) {
        return true;
    }
    let corners = [
        Point {
            x: rect.x,
            y: rect.y,
        },
        Point {
            x: rect.x + rect.width,
            y: rect.y,
        },
        Point {
            x: rect.x + rect.width,
            y: rect.y + rect.height,
        },
        Point {
            x: rect.x,
            y: rect.y + rect.height,
        },
    ];
    (0..4).any(|i| {
        let a = corners[i];
        let b = corners[(i + 1) % 4];
        segments_cross(p1, p2, a, b)
    })
}

fn point_in(p: Point, rect: Rect) -> bool {
    (rect.x..=rect.x + rect.width).contains(&p.x) && p.y >= rect.y && p.y <= rect.y + rect.height
}

fn segments_cross(p1: Point, p2: Point, p3: Point, p4: Point) -> bool {
    let d =
        |a: Point, b: Point, c: Point| ((c.x - a.x) * (b.y - a.y)) - ((b.x - a.x) * (c.y - a.y));
    let d1 = d(p3, p4, p1);
    let d2 = d(p3, p4, p2);
    let d3 = d(p1, p2, p3);
    let d4 = d(p1, p2, p4);
    ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0))
        && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
}

/// elkrb's `minimize_bends`: connect each point to the furthest
/// point it can see, then drop the endpoints from the result.
fn simplify(path: &[Point], obstacles: &[Rect], start: Point, goal: Point) -> Vec<Point> {
    if path.len() < 3 {
        return Vec::new();
    }
    let bends = &path[1..path.len() - 1];
    if bends.is_empty() {
        return Vec::new();
    }
    let mut all = vec![start];
    all.extend_from_slice(bends);
    all.push(goal);
    let mut simplified = vec![all[0]];
    let mut i = 0;
    while i < all.len() - 1 {
        let mut jumped = false;
        for j in ((i + 2)..all.len()).rev() {
            if !obstacles
                .iter()
                .any(|rect| segment_hits(all[i], all[j], *rect))
            {
                simplified.push(all[j]);
                i = j;
                jumped = true;
                break;
            }
        }
        if !jumped {
            simplified.push(all[i + 1]);
            i += 1;
        }
    }
    simplified[1..simplified.len().saturating_sub(1)].to_vec()
}
