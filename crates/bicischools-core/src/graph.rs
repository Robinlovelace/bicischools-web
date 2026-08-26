use crate::cost::EdgeAttributes;
use crate::osm::{OsmElement, OverpassResponse};
use geo::{Coord, LineString};
use rstar::{PointDistance, RTree, RTreeObject, AABB};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Approximate radius of Earth in meters
pub const EARTH_RADIUS_M: f64 = 6_371_000.0;

/// Haversine distance between two coordinates in degrees [lng, lat]
pub fn haversine_distance(lng1: f64, lat1: f64, lng2: f64, lat2: f64) -> f64 {
    let d_lat = (lat2 - lat1).to_radians();
    let d_lng = (lng2 - lng1).to_radians();
    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();

    let a = (d_lat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (d_lng / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    EARTH_RADIUS_M * c
}

/// Calculate the total length in meters of a LineString
pub fn linestring_length_m(line: &LineString<f64>) -> f64 {
    line.lines()
        .map(|segment| {
            haversine_distance(segment.start.x, segment.start.y, segment.end.x, segment.end.y)
        })
        .sum()
}

/// Spatial node wrapper for RTree indexing
#[derive(Debug, Clone, Copy)]
pub struct IndexedNode {
    pub node_id: usize, // index into graph.nodes
    pub osm_id: i64,
    pub lng: f64,
    pub lat: f64,
}

impl RTreeObject for IndexedNode {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_point([self.lng, self.lat])
    }
}

impl PointDistance for IndexedNode {
    fn distance_2(&self, point: &[f64; 2]) -> f64 {
        let dx = self.lng - point[0];
        let dy = self.lat - point[1];
        dx * dx + dy * dy
    }
}

/// Graph Node representing a road junction or geometry point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: usize,
    pub osm_id: i64,
    pub lng: f64,
    pub lat: f64,
    pub outgoing_edges: Vec<usize>, // indices into graph.edges
}

/// Graph Edge representing a street segment between two nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: usize,
    pub way_id: i64,
    pub from_node: usize,
    pub to_node: usize,
    pub geometry: LineString<f64>,
    pub length_m: f64,
    pub attributes: EdgeAttributes,
}

/// Spatial road graph for routing and network analysis
pub struct StreetGraph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub osm_id_to_node: HashMap<i64, usize>,
    pub spatial_index: RTree<IndexedNode>,
    pub bounds: [f64; 4], // [min_lng, min_lat, max_lng, max_lat]
}

impl StreetGraph {
    /// Builds a StreetGraph from an Overpass response
    pub fn from_overpass(response: &OverpassResponse) -> Self {
        let mut raw_nodes: HashMap<i64, (f64, f64)> = HashMap::new();
        let mut node_used_count: HashMap<i64, usize> = HashMap::new();
        let mut ways = Vec::new();

        // 1. Collect nodes
        for el in &response.elements {
            match el {
                OsmElement::Node { id, lat, lon, .. } => {
                    raw_nodes.insert(*id, (*lon, *lat));
                }
                OsmElement::Way { id, nodes, tags, .. } => {
                    let highway = tags.get("highway").map(|s| s.as_str()).unwrap_or_default();
                    if highway.is_empty()
                        || highway == "construction"
                        || highway == "proposed"
                        || highway == "corridor"
                        || highway == "steps"
                    {
                        continue;
                    }
                    for &nid in nodes {
                        *node_used_count.entry(nid).or_insert(0) += 1;
                    }
                    ways.push((*id, nodes.clone(), tags.clone()));
                }
                _ => {}
            }
        }

        let mut nodes = Vec::new();
        let mut osm_id_to_node = HashMap::new();
        let mut min_lng = f64::INFINITY;
        let mut min_lat = f64::INFINITY;
        let mut max_lng = f64::NEG_INFINITY;
        let mut max_lat = f64::NEG_INFINITY;

        // 2. Identify intersection/end nodes
        for (osm_id, &(lng, lat)) in &raw_nodes {
            if node_used_count.get(osm_id).copied().unwrap_or(0) > 0 {
                let idx = nodes.len();
                nodes.push(Node {
                    id: idx,
                    osm_id: *osm_id,
                    lng,
                    lat,
                    outgoing_edges: Vec::new(),
                });
                osm_id_to_node.insert(*osm_id, idx);

                if lng < min_lng { min_lng = lng; }
                if lat < min_lat { min_lat = lat; }
                if lng > max_lng { max_lng = lng; }
                if lat > max_lat { max_lat = lat; }
            }
        }

        let mut edges = Vec::new();

        // 3. Build edges between consecutive nodes in ways
        for (way_id, way_nodes, tags) in ways {
            if way_nodes.len() < 2 {
                continue;
            }

            for i in 0..way_nodes.len() - 1 {
                let n1_osm = way_nodes[i];
                let n2_osm = way_nodes[i + 1];

                let n1_idx = match osm_id_to_node.get(&n1_osm) {
                    Some(&idx) => idx,
                    None => continue,
                };
                let n2_idx = match osm_id_to_node.get(&n2_osm) {
                    Some(&idx) => idx,
                    None => continue,
                };

                let (p1_lng, p1_lat) = (nodes[n1_idx].lng, nodes[n1_idx].lat);
                let (p2_lng, p2_lat) = (nodes[n2_idx].lng, nodes[n2_idx].lat);

                let line = LineString::new(vec![
                    Coord { x: p1_lng, y: p1_lat },
                    Coord { x: p2_lng, y: p2_lat },
                ]);
                let length_m = linestring_length_m(&line);
                if length_m <= 0.001 {
                    continue;
                }

                let attrs = EdgeAttributes::from_tags(&tags, length_m);

                // Forward edge
                let edge_idx = edges.len();
                edges.push(Edge {
                    id: edge_idx,
                    way_id,
                    from_node: n1_idx,
                    to_node: n2_idx,
                    geometry: line.clone(),
                    length_m,
                    attributes: attrs.clone(),
                });
                nodes[n1_idx].outgoing_edges.push(edge_idx);

                // Backward edge (if not strictly one-way for bicycles)
                if !attrs.is_one_way {
                    let rev_line = LineString::new(vec![
                        Coord { x: p2_lng, y: p2_lat },
                        Coord { x: p1_lng, y: p1_lat },
                    ]);
                    let rev_edge_idx = edges.len();
                    edges.push(Edge {
                        id: rev_edge_idx,
                        way_id,
                        from_node: n2_idx,
                        to_node: n1_idx,
                        geometry: rev_line,
                        length_m,
                        attributes: attrs,
                    });
                    nodes[n2_idx].outgoing_edges.push(rev_edge_idx);
                }
            }
        }

        // 4. Build spatial index
        let indexed_nodes: Vec<IndexedNode> = nodes
            .iter()
            .map(|n| IndexedNode {
                node_id: n.id,
                osm_id: n.osm_id,
                lng: n.lng,
                lat: n.lat,
            })
            .collect();
        let spatial_index = RTree::bulk_load(indexed_nodes);

        if min_lng.is_infinite() {
            min_lng = 0.0; min_lat = 0.0; max_lng = 0.0; max_lat = 0.0;
        }

        Self {
            nodes,
            edges,
            osm_id_to_node,
            spatial_index,
            bounds: [min_lng, min_lat, max_lng, max_lat],
        }
    }

    /// Finds the nearest node ID to a given [lng, lat] coordinate
    pub fn find_nearest_node(&self, lng: f64, lat: f64) -> Option<usize> {
        self.spatial_index
            .nearest_neighbor(&[lng, lat])
            .map(|n| n.node_id)
    }
}
