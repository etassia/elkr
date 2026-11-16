//! The DIM adapter: the Diagram Interchange Model rides the same
//! layout as everything else. A DIM 0.1 document's `graph` is the ELK
//! JSON graph elkr speaks, so the adapter lifts it out, lays it out
//! (honoring the document's `layoutOptions` — DIM's default
//! `elk.algorithm` is layered, like ELK's), and merges the positions
//! and edge sections back into the document, leaving every field the
//! engine does not speak — `content` blocks, `style`, `notation` —
//! exactly as it arrived.

use crate::elk::ElkGraph;
use serde_json::Value;

pub const SUPPORTED_VERSION: &str = "0.1";

#[derive(Debug, thiserror::Error)]
pub enum DimError {
    #[error("DIM version {0} is not supported; this adapter speaks {SUPPORTED_VERSION}")]
    Unsupported(String),
    #[error("invalid DIM document: {0}")]
    Invalid(String),
    #[error(transparent)]
    Layout(#[from] super::LayoutError),
    #[error("json: {0}")]
    Json(String),
}

/// Lays out a DIM 0.1 document and returns the document JSON with the
/// positions and edge sections merged in.
pub fn layout_document(json: &str) -> Result<String, DimError> {
    let mut document: Value =
        serde_json::from_str(json).map_err(|e| DimError::Json(e.to_string()))?;
    if document.get("dim").and_then(Value::as_str) != Some(SUPPORTED_VERSION) {
        let version = document
            .get("dim")
            .and_then(Value::as_str)
            .unwrap_or("missing")
            .to_string();
        return Err(DimError::Unsupported(version));
    }
    let mut graph_value = document
        .get_mut("graph")
        .ok_or_else(|| DimError::Invalid("the document carries no graph".to_string()))?
        .take();
    let mut graph: ElkGraph = serde_json::from_value(graph_value.clone())
        .map_err(|e| DimError::Invalid(e.to_string()))?;
    crate::apply(&mut graph)?;
    let laid = serde_json::to_value(&graph).map_err(|e| DimError::Json(e.to_string()))?;
    merge_geometry(&mut graph_value, &laid);
    *document.get_mut("graph").expect("checked above") = graph_value;
    serde_json::to_string(&document).map_err(|e| DimError::Json(e.to_string()))
}

/// Copy the layout output — node x/y and edge sections — into the
/// original graph value; everything else the document carried stays.
fn merge_geometry(original: &mut Value, laid: &Value) {
    if let (Some(children), Some(laid_children)) = (
        original.get_mut("children").and_then(Value::as_array_mut),
        laid.get("children"),
    ) {
        for child in children {
            let Some(id) = child.get("id").and_then(Value::as_str) else {
                continue;
            };
            let Some(laid_child) = laid_children.as_array().and_then(|c| {
                c.iter()
                    .find(|c| c.get("id").and_then(Value::as_str) == Some(id))
            }) else {
                continue;
            };
            for field in ["x", "y", "width", "height"] {
                if let Some(value) = laid_child.get(field) {
                    child[field] = value.clone();
                }
            }
        }
    }
    if let (Some(edges), Some(laid_edges)) = (
        original.get_mut("edges").and_then(Value::as_array_mut),
        laid.get("edges"),
    ) {
        for edge in edges {
            let Some(id) = edge.get("id").and_then(Value::as_str) else {
                continue;
            };
            let Some(laid_edge) = laid_edges.as_array().and_then(|e| {
                e.iter()
                    .find(|e| e.get("id").and_then(Value::as_str) == Some(id))
            }) else {
                continue;
            };
            if let Some(sections) = laid_edge.get("sections") {
                edge["sections"] = sections.clone();
            }
        }
    }
}
