//! ELK JSON compatibility model.
//!
//! The interchange format is the ELK JSON graph that elkjs and elkrb speak:
//! nodes carry geometry, edges reference endpoint ids, and layout options
//! ride on the root as a string-to-string map. Positions are absent on
//! input and present on output; elkr fills them in.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const DEFAULT_NODE_SPACING: f64 = 20.0;
pub const DEFAULT_LAYER_SPACING: f64 = 60.0;
pub const DEFAULT_NODE_WIDTH: f64 = 40.0;
pub const DEFAULT_NODE_HEIGHT: f64 = 40.0;

/// A layout-option map: ELK option keys to string values.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(transparent)]
pub struct LayoutOptions(pub BTreeMap<String, String>);

impl LayoutOptions {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn insert(&mut self, key: &str, value: &str) {
        self.0.insert(key.to_string(), value.to_string());
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.0.get(key)
    }

    pub fn get_parsed<T: std::str::FromStr>(&self, key: &str) -> Option<T> {
        self.0.get(key).and_then(|v| v.parse().ok())
    }

    pub fn direction(&self) -> Direction {
        match self.0.get("elk.direction").map(String::as_str) {
            Some("DOWN") => Direction::Down,
            Some("UP") => Direction::Up,
            Some("LEFT") => Direction::Left,
            _ => Direction::Right,
        }
    }

    /// The gap between nodes inside one layer (ELK: elk.spacing.nodeNode;
    /// elkrb carries the same knob).
    pub fn node_spacing(&self) -> f64 {
        self.get_parsed("elk.spacing.nodeNode")
            .unwrap_or(DEFAULT_NODE_SPACING)
    }

    /// The gap between layers (ELK:
    /// elk.layered.spacing.nodeNodeBetweenLayers, falling back to
    /// elk.spacing.nodeNode).
    pub fn layer_spacing(&self) -> f64 {
        self.get_parsed("elk.layered.spacing.nodeNodeBetweenLayers")
            .or_else(|| self.get_parsed("elk.spacing.nodeNode"))
            .unwrap_or(DEFAULT_LAYER_SPACING)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Right,
    Left,
    Down,
    Up,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElkGraph {
    pub id: String,
    #[serde(default)]
    pub children: Vec<ElkNode>,
    #[serde(default)]
    pub edges: Vec<ElkEdge>,
    #[serde(default, skip_serializing_if = "LayoutOptions::is_empty")]
    pub layout_options: LayoutOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElkNode {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<ElkNode>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<ElkLabel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElkLabel {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElkEdge {
    pub id: String,
    #[serde(default)]
    pub sources: Vec<String>,
    #[serde(default)]
    pub targets: Vec<String>,
    /// Layout output: the routed sections of the edge.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sections: Vec<EdgeSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EdgeSection {
    pub id: String,
    pub start_point: Point,
    pub end_point: Point,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bend_points: Vec<Point>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub incoming_shape: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outgoing_shape: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl ElkGraph {
    /// The flat child list is the layout surface of v0.1; nested
    /// hierarchies are processed per level in a later version.
    pub fn flat_children(&self) -> &[ElkNode] {
        &self.children
    }

    pub fn find_node(&self, id: &str) -> Option<&ElkNode> {
        self.children.iter().find(|n| n.id == id)
    }

    pub fn node_mut(&mut self, id: &str) -> Option<&mut ElkNode> {
        self.children.iter_mut().find(|n| n.id == id)
    }
}
