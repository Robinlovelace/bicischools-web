use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Overpass API Response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverpassResponse {
    #[serde(default)]
    pub elements: Vec<OsmElement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OsmElement {
    #[serde(rename = "node")]
    Node {
        id: i64,
        lat: f64,
        lon: f64,
        #[serde(default)]
        tags: HashMap<String, String>,
    },
    #[serde(rename = "way")]
    Way {
        id: i64,
        #[serde(default)]
        nodes: Vec<i64>,
        #[serde(default)]
        tags: HashMap<String, String>,
        #[serde(default)]
        geometry: Option<Vec<OsmGeometryCoord>>,
    },
    #[serde(rename = "relation")]
    Relation {
        id: i64,
        #[serde(default)]
        tags: HashMap<String, String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsmGeometryCoord {
    pub lat: f64,
    pub lon: f64,
}

/// Parses an Overpass API JSON string into an OverpassResponse
pub fn parse_overpass_json(json_str: &str) -> anyhow::Result<OverpassResponse> {
    let resp: OverpassResponse = serde_json::from_str(json_str)?;
    Ok(resp)
}
