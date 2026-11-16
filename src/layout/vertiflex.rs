//! The VertiFlex algorithm: the port of elkrb's `VertiFlex`.
//!
//! Nodes distribute into vertical columns (round-robin when
//! `vertiflex.balanceColumns`, sequential slices otherwise) and stack
//! inside their column. Column widths flex to the widest node; the
//! gaps come from `vertiflex.columnSpacing` (50) and
//! `vertiflex.verticalSpacing` (30, overridden by
//! `elk.spacing.nodeNode`). One deviation, deliberate: elkrb reads
//! balanceColumns as a Ruby string, where "false" is truthy — the
//! option never worked there. elkr parses it as a boolean.

use super::{apply_padding, straight_sections, LayoutAlgorithm, LayoutError};
use crate::elk::ElkGraph;
use std::str::FromStr;

pub struct VertiFlex;

const DEFAULT_COLUMNS: usize = 3;
const DEFAULT_COLUMN_SPACING: f64 = 50.0;
const DEFAULT_VERTICAL_SPACING: f64 = 30.0;

fn option<T: FromStr>(graph: &ElkGraph, key: &str) -> Option<T> {
    graph.layout_options.get_parsed::<T>(key)
}

impl LayoutAlgorithm for VertiFlex {
    fn name(&self) -> &'static str {
        "vertiflex"
    }

    fn layout(&self, graph: &mut ElkGraph) -> Result<(), LayoutError> {
        if graph.children.is_empty() {
            return Ok(());
        }
        if graph.children.len() == 1 {
            graph.children[0].x = Some(0.0);
            graph.children[0].y = Some(0.0);
            apply_padding(graph);
            return straight_sections(graph);
        }

        let columns = option::<usize>(graph, "vertiflex.columnCount")
            .unwrap_or(DEFAULT_COLUMNS)
            .max(1);
        let column_spacing =
            option::<f64>(graph, "vertiflex.columnSpacing").unwrap_or(DEFAULT_COLUMN_SPACING);
        let vertical_spacing = option::<f64>(graph, "elk.spacing.nodeNode")
            .or_else(|| option::<f64>(graph, "vertiflex.verticalSpacing"))
            .unwrap_or(DEFAULT_VERTICAL_SPACING);
        let balance = option::<bool>(graph, "vertiflex.balanceColumns").unwrap_or(true);

        let count = graph.children.len();
        let mut buckets: Vec<Vec<usize>> = vec![Vec::new(); columns];
        if balance {
            for i in 0..count {
                buckets[i % columns].push(i);
            }
        } else {
            let per_column = count.div_ceil(columns);
            for i in 0..count {
                buckets[i / per_column].push(i);
            }
        }

        let mut column_x = 0.0;
        for bucket in buckets.iter().filter(|b| !b.is_empty()) {
            let column_width = bucket
                .iter()
                .filter_map(|&i| graph.children[i].width)
                .fold(0.0f64, f64::max);
            let column_width = if column_width == 0.0 {
                100.0
            } else {
                column_width
            };

            let mut y = 0.0;
            for &i in bucket {
                let node = &mut graph.children[i];
                node.x = Some(column_x);
                node.y = Some(y);
                y += node.height.unwrap_or(0.0) + vertical_spacing;
            }
            column_x += column_width + column_spacing;
        }

        apply_padding(graph);
        straight_sections(graph)
    }
}
