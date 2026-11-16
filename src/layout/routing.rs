//! Edge routing and label placement: elkrb's per-edge orthogonal
//! routing and the label pass.
//!
//! An edge whose `edge.routing` option is `orthogonal` routes through
//! two bends at the horizontal midpoint — elkrb's
//! `add_orthogonal_bend_points`. The labels an edge carries then take
//! their positions on the route: each label centers on the path's
//! arc-length midpoint and stacks above the line — elkrb's
//! `place_edge_labels` with CENTER placement.

use crate::elk::{ElkEdge, Point};

/// The two bends of elkrb's orthogonal route: out horizontally to the
/// midpoint, then vertically to the target's line.
pub fn orthogonal_bends(start: Point, end: Point) -> Vec<Point> {
    let mid_x = (start.x + end.x) / 2.0;
    vec![
        Point {
            x: mid_x,
            y: start.y,
        },
        Point { x: mid_x, y: end.y },
    ]
}

/// elkrb's `calculate_edge_center`: the point at half the total path
/// length, interpolated on the segment that reaches it.
pub fn path_center(start: &Point, bends: &[Point], end: &Point) -> Point {
    let points: Vec<Point> = std::iter::once(*start)
        .chain(bends.iter().copied())
        .chain(std::iter::once(*end))
        .collect();
    let mut lengths = Vec::with_capacity(points.len() - 1);
    let mut total = 0.0;
    for pair in points.windows(2) {
        let length = ((pair[1].x - pair[0].x).powi(2) + (pair[1].y - pair[0].y).powi(2)).sqrt();
        lengths.push(length);
        total += length;
    }
    let target = total / 2.0;
    let mut walked = 0.0;
    for (i, length) in lengths.iter().enumerate() {
        if walked + length >= target {
            let ratio = (target - walked) / length;
            let (p1, p2) = (&points[i], &points[i + 1]);
            return Point {
                x: p1.x + ratio * (p2.x - p1.x),
                y: p1.y + ratio * (p2.y - p1.y),
            };
        }
        walked += length;
    }
    points[points.len() / 2]
}

/// elkrb's CENTER placement: each label centers on the path and
/// stacks above the line, one label height apart.
pub fn place_labels(edge: &mut ElkEdge) {
    let Some(first) = edge.sections.first().cloned() else {
        return;
    };
    for (index, label) in edge.labels.iter_mut().enumerate() {
        let center = path_center(&first.start_point, &first.bend_points, &first.end_point);
        let height = label.height.unwrap_or(0.0);
        label.x = Some(center.x);
        label.y = Some(first.start_point.y - height - 5.0 + index as f64 * (height + 2.0));
    }
}

/// Whether the edge asked for the orthogonal route.
pub fn orthogonal(edge: &ElkEdge) -> bool {
    edge.layout_options.get("edge.routing").map(String::as_str) == Some("orthogonal")
}
