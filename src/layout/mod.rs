//! The layout algorithm surface: one trait, a registry by name.

pub mod box_layout;
pub mod disco;
pub mod fixed;
pub mod force;
pub mod layered;
pub mod libavoid;
pub mod mrtree;
pub mod radial;
pub mod random;
pub mod rectpacking;
pub mod spore_compaction;
pub mod spore_overlap;
pub mod stress;
pub mod topdownpacking;
pub mod vertiflex;

use crate::elk::{EdgeSection, ElkGraph, Point};
use std::collections::BTreeMap;

/// elkrb's padding: every algorithm shifts its result so the bounding
/// box starts here.
pub(crate) const PADDING: f64 = 12.0;

/// SplitMix64: the fixed-seed stream that stands in for elkrb's
/// `Kernel#rand`, so scattered layouts are identical on every run.
pub(crate) struct Scatter(pub u64);

impl Scatter {
    pub fn next_unit(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        ((z >> 11) as f64) / (1u64 << 53) as f64
    }
}

/// elkrb's `apply_padding`: shift the laid-out nodes so the bounding
/// box starts at (PADDING, PADDING).
pub(crate) fn apply_padding(graph: &mut ElkGraph) {
    if graph.children.is_empty() {
        return;
    }
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
        node.x = Some(node.x.unwrap_or(0.0) - min_x + PADDING);
        node.y = Some(node.y.unwrap_or(0.0) - min_y + PADDING);
    }
}

/// elkrb's default edge routing: straight sections, center to center.
/// Edges whose endpoints are not on the flat surface skip, as elkrb's
/// EdgeRouter does; self loops skip.
pub(crate) fn straight_sections(graph: &mut ElkGraph) -> Result<(), LayoutError> {
    let index_of: BTreeMap<String, usize> = graph
        .children
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.clone(), i))
        .collect();
    for e in graph.edges.iter_mut() {
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
        let sn = &graph.children[si];
        let tn = &graph.children[ti];
        let center = |n: &crate::elk::ElkNode| Point {
            x: n.x.unwrap_or(0.0) + n.width.unwrap_or(crate::elk::DEFAULT_NODE_WIDTH) / 2.0,
            y: n.y.unwrap_or(0.0) + n.height.unwrap_or(crate::elk::DEFAULT_NODE_HEIGHT) / 2.0,
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

#[derive(Debug, thiserror::Error)]
pub enum LayoutError {
    #[error("edge {edge} references unknown node {node}")]
    UnknownNode { edge: String, node: String },
    #[error("unknown layout algorithm {0}; available: {1}")]
    UnknownAlgorithm(String, String),
    #[error("layout of {0} failed: {1}")]
    Failed(String, String),
}

pub trait LayoutAlgorithm: Send + Sync {
    fn name(&self) -> &'static str;
    fn layout(&self, graph: &mut ElkGraph) -> Result<(), LayoutError>;
}

/// The algorithm registry, keyed by the ELK option value of
/// `elk.algorithm`. The list is part of the crate's compatibility
/// surface.
pub fn registry() -> std::collections::BTreeMap<&'static str, Box<dyn LayoutAlgorithm>> {
    let mut m: std::collections::BTreeMap<&'static str, Box<dyn LayoutAlgorithm>> =
        std::collections::BTreeMap::new();
    m.insert("box", Box::new(box_layout::BoxLayout));
    m.insert("disco", Box::new(disco::Disco));
    m.insert("fixed", Box::new(fixed::Fixed));
    m.insert("force", Box::new(force::Force));
    m.insert("layered", Box::new(layered::Layered));
    m.insert("libavoid", Box::new(libavoid::Libavoid));
    m.insert("mrtree", Box::new(mrtree::MRTree));
    m.insert("radial", Box::new(radial::Radial));
    m.insert("random", Box::new(random::Random));
    m.insert("rectpacking", Box::new(rectpacking::RectPacking));
    m.insert(
        "spore-compaction",
        Box::new(spore_compaction::SporeCompaction),
    );
    m.insert("spore-overlap", Box::new(spore_overlap::SporeOverlap));
    m.insert("stress", Box::new(stress::Stress));
    m.insert("topdownpacking", Box::new(topdownpacking::TopdownPacking));
    m.insert("vertiflex", Box::new(vertiflex::VertiFlex));
    m
}

pub fn by_name(name: &str) -> Result<Box<dyn LayoutAlgorithm>, LayoutError> {
    match name {
        "box" => Ok(Box::new(box_layout::BoxLayout)),
        "disco" => Ok(Box::new(disco::Disco)),
        "fixed" => Ok(Box::new(fixed::Fixed)),
        "force" => Ok(Box::new(force::Force)),
        "layered" => Ok(Box::new(layered::Layered)),
        "libavoid" => Ok(Box::new(libavoid::Libavoid)),
        "mrtree" => Ok(Box::new(mrtree::MRTree)),
        "radial" => Ok(Box::new(radial::Radial)),
        "random" => Ok(Box::new(random::Random)),
        "rectpacking" => Ok(Box::new(rectpacking::RectPacking)),
        "spore-compaction" => Ok(Box::new(spore_compaction::SporeCompaction)),
        "spore-overlap" => Ok(Box::new(spore_overlap::SporeOverlap)),
        "stress" => Ok(Box::new(stress::Stress)),
        "topdownpacking" => Ok(Box::new(topdownpacking::TopdownPacking)),
        "vertiflex" => Ok(Box::new(vertiflex::VertiFlex)),
        other => {
            let r = registry();
            Err(LayoutError::UnknownAlgorithm(
                other.to_string(),
                r.keys()
                    .map(|k| k.to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
            ))
        }
    }
}

/// Apply the algorithm named in the graph's `elk.algorithm` option
/// (default: `layered`, the ELK default too).
pub fn apply(graph: &mut ElkGraph) -> Result<(), LayoutError> {
    let name = graph
        .layout_options
        .get("elk.algorithm")
        .map(String::as_str)
        .unwrap_or("layered");
    by_name(name)?.layout(graph)
}
