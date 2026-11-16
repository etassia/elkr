//! The DISCO algorithm: the port of elkrb's `Disco`.
//!
//! Disconnected-component layout: find the connected components (BFS
//! over the edges), lay each one out independently with
//! `disco.componentAlgorithm` (layered by default; an unknown name
//! leaves the component where it started, as in elkrb), then arrange
//! the components in a row, a column, or a grid (`disco.componentSpacing`,
//! 20). Edges route fresh on the final positions, the way elkrb's
//! pipeline re-routes after the algorithm returns. The inner layout
//! sees no options — elkrb builds its component graph bare, so the
//! inner algorithm runs on its defaults.

use super::{apply_padding, straight_sections, LayoutAlgorithm, LayoutError};
use crate::elk::ElkGraph;

pub struct Disco;

const DEFAULT_COMPONENT_SPACING: f64 = 20.0;

impl LayoutAlgorithm for Disco {
    fn name(&self) -> &'static str {
        "disco"
    }

    fn layout(&self, graph: &mut ElkGraph) -> Result<(), LayoutError> {
        if graph.children.is_empty() {
            return Ok(());
        }

        let components = self.components(graph);

        let inner_name = graph
            .layout_options
            .get("disco.componentAlgorithm")
            .map(String::as_str)
            .unwrap_or("layered");
        let inner = crate::layout::by_name(inner_name).ok();
        if let Some(algorithm) = inner {
            for (nodes, edges) in &components {
                if nodes.is_empty() {
                    continue;
                }
                let mut component = ElkGraph {
                    id: "component".to_string(),
                    children: nodes.iter().map(|&i| graph.children[i].clone()).collect(),
                    edges: edges.iter().map(|&i| graph.edges[i].clone()).collect(),
                    layout_options: Default::default(),
                };
                algorithm.layout(&mut component)?;
                for (slot, laid) in nodes.iter().zip(&component.children) {
                    graph.children[*slot].x = laid.x;
                    graph.children[*slot].y = laid.y;
                }
            }
        }

        let spacing = graph
            .layout_options
            .get_parsed::<f64>("disco.componentSpacing")
            .unwrap_or(DEFAULT_COMPONENT_SPACING);
        let arrangement = graph
            .layout_options
            .get("disco.componentArrangement")
            .map(String::as_str)
            .unwrap_or("row");

        match arrangement {
            "grid" => arrange_in_grid(&components, graph, spacing),
            "column" => arrange_in_column(&components, graph, spacing),
            _ => arrange_in_row(&components, graph, spacing),
        }

        apply_padding(graph);
        for e in &mut graph.edges {
            e.sections.clear();
        }
        straight_sections(graph)
    }
}

impl Disco {
    /// Connected components as (node indices, edge indices) pairs.
    fn components(&self, graph: &ElkGraph) -> Vec<(Vec<usize>, Vec<usize>)> {
        let mut visited = vec![false; graph.children.len()];
        let mut components = Vec::new();
        for start in 0..graph.children.len() {
            if visited[start] {
                continue;
            }
            let mut nodes = Vec::new();
            let mut edges = Vec::new();
            let mut queue = vec![start];
            visited[start] = true;
            while let Some(current) = queue.pop() {
                nodes.push(current);
                let id = graph.children[current].id.as_str();
                for (ei, e) in graph.edges.iter().enumerate() {
                    let touches = e.sources.iter().chain(&e.targets).any(|s| s.as_str() == id);
                    if !touches {
                        continue;
                    }
                    if !edges.contains(&ei) {
                        edges.push(ei);
                    }
                    for endpoint in e.sources.iter().chain(&e.targets) {
                        if let Some(i) = graph.children.iter().position(|n| n.id == *endpoint) {
                            if !visited[i] {
                                visited[i] = true;
                                queue.push(i);
                            }
                        }
                    }
                }
            }
            components.push((nodes, edges));
        }
        components
    }
}

fn bounds(graph: &ElkGraph, nodes: &[usize]) -> (f64, f64, f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for &i in nodes {
        let n = &graph.children[i];
        let x = n.x.unwrap_or(0.0);
        let y = n.y.unwrap_or(0.0);
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x + n.width.unwrap_or(0.0));
        max_y = max_y.max(y + n.height.unwrap_or(0.0));
    }
    if !nodes.is_empty() {
        return (min_x, min_y, max_x, max_y);
    }
    (0.0, 0.0, 0.0, 0.0)
}

fn arrange_in_row(components: &[(Vec<usize>, Vec<usize>)], graph: &mut ElkGraph, spacing: f64) {
    let mut x_offset = 0.0;
    for (nodes, _) in components {
        let (min_x, _, max_x, _) = bounds(graph, nodes);
        let width = max_x - min_x;
        for &i in nodes {
            let n = &mut graph.children[i];
            n.x = Some(n.x.unwrap_or(0.0) + x_offset - min_x);
        }
        x_offset += width + spacing;
    }
}

fn arrange_in_column(components: &[(Vec<usize>, Vec<usize>)], graph: &mut ElkGraph, spacing: f64) {
    let mut y_offset = 0.0;
    for (nodes, _) in components {
        let (_, min_y, _, max_y) = bounds(graph, nodes);
        let height = max_y - min_y;
        for &i in nodes {
            let n = &mut graph.children[i];
            n.y = Some(n.y.unwrap_or(0.0) + y_offset - min_y);
        }
        y_offset += height + spacing;
    }
}

fn arrange_in_grid(components: &[(Vec<usize>, Vec<usize>)], graph: &mut ElkGraph, spacing: f64) {
    if components.is_empty() {
        return;
    }
    let cols = ((components.len() as f64).sqrt().ceil()) as usize;
    let rows = components.len().div_ceil(cols);

    let mut y_offset = 0.0;
    for row in 0..rows {
        let mut x_offset = 0.0f64;
        let mut max_row_height = 0.0f64;
        for col in 0..cols {
            let index = row * cols + col;
            let Some((nodes, _)) = components.get(index) else {
                break;
            };
            let (min_x, min_y, max_x, max_y) = bounds(graph, nodes);
            for &i in nodes {
                let n = &mut graph.children[i];
                n.x = Some(n.x.unwrap_or(0.0) + x_offset - min_x);
                n.y = Some(n.y.unwrap_or(0.0) + y_offset - min_y);
            }
            x_offset += max_x - min_x + spacing;
            max_row_height = max_row_height.max(max_y - min_y);
        }
        y_offset += max_row_height + spacing;
    }
}
