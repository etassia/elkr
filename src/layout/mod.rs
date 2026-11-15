//! The layout algorithm surface: one trait, a registry by name.

pub mod layered;

use crate::elk::ElkGraph;

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
    m.insert("layered", Box::new(layered::Layered));
    m
}

pub fn by_name(name: &str) -> Result<Box<dyn LayoutAlgorithm>, LayoutError> {
    match name {
        "layered" => Ok(Box::new(layered::Layered)),
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
