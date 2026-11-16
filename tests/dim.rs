//! The DIM adapter tests: a document in, a laid-out document out.

use elkr::dim::{layout_document, DimError};

fn document() -> String {
    serde_json::json!({
        "dim": "0.1",
        "notation": "generic",
        "graph": {
            "id": "root",
            "layoutOptions": { "elk.algorithm": "layered", "elk.direction": "RIGHT" },
            "children": [
                { "id": "a", "width": 100, "height": 40,
                  "content": [{ "kind": "label", "text": "Hard", "role": "title" }] },
                { "id": "b", "width": 100, "height": 40,
                  "content": [{ "kind": "label", "text": "Easy", "role": "title" }] }
            ],
            "edges": [
                { "id": "e", "sources": ["a"], "targets": ["b"],
                  "style": "association" }
            ]
        }
    })
    .to_string()
}

#[test]
fn a_document_lays_out_and_keeps_its_provenance() {
    let out = layout_document(&document()).unwrap();
    let doc: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(doc["dim"], "0.1");
    assert_eq!(doc["notation"], "generic");
    let children = doc["graph"]["children"].as_array().unwrap();
    assert!(children[0]["x"].is_number(), "the positions are missing");
    let sections = doc["graph"]["edges"][0]["sections"]
        .as_array()
        .expect("the edge sections are missing");
    assert!(sections[0]["startPoint"]["x"].as_f64().unwrap().is_finite());
    // the content blocks the engine does not touch ride along untouched
    assert_eq!(children[0]["content"][0]["text"], "Hard");
}

#[test]
fn the_document_is_deterministic() {
    let a = layout_document(&document()).unwrap();
    let b = layout_document(&document()).unwrap();
    assert_eq!(a, b);
}

#[test]
fn a_foreign_version_refuses() {
    let mut foreign: serde_json::Value = serde_json::from_str(&document()).unwrap();
    foreign["dim"] = serde_json::json!("9.9");
    let foreign = foreign.to_string();
    match layout_document(&foreign) {
        Err(DimError::Unsupported(v)) => assert_eq!(v, "9.9"),
        other => panic!("expected Unsupported, got {other:?}"),
    }
}

#[test]
fn a_document_without_a_graph_refuses() {
    match layout_document(r#"{ "dim": "0.1", "notation": "generic" }"#) {
        Err(DimError::Invalid(_)) => {}
        other => panic!("expected Invalid, got {other:?}"),
    }
}

#[test]
fn the_document_steers_the_algorithm() {
    // force on the same document yields different coordinates than
    // layered: the document's layoutOptions reach the engine
    let layered = layout_document(&document()).unwrap();
    let mut forced: serde_json::Value = serde_json::from_str(&document()).unwrap();
    forced["graph"]["layoutOptions"]["elk.algorithm"] = serde_json::json!("random");
    let random = layout_document(&forced.to_string()).unwrap();
    assert_ne!(layered, random);
}
