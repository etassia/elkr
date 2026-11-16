//! elkr: deterministic graph layout, ported from metanorma/elkrb.
//!
//! The crate reads and writes the ELK JSON graph format (the same
//! interchange that elkjs and elkrb speak), computes node positions with
//! ELK's algorithms, and routes edges. Layout is deterministic: the same
//! input graph and options always produce the same coordinates, which is
//! the property Etassia's byte-identity contract depends on.

pub mod dim;
pub mod elk;
pub mod layout;

pub use elk::{Direction, EdgeSection, ElkEdge, ElkGraph, ElkNode, LayoutOptions, Point};
pub use layout::{apply, by_name, registry, LayoutAlgorithm, LayoutError};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
