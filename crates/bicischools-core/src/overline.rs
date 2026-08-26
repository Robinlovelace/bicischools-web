use crate::graph::StreetGraph;
use crate::router::RouteResult;
use geo::LineString;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSegment {
    pub edge_id: usize,
    pub way_id: i64,
    pub name: Option<String>,
    pub highway: String,
    pub length_m: f64,
    pub quietness: f64,
    pub num_students: f64,
    pub bicycle_godutch: f64,
    pub route_count: usize,
    pub geometry: LineString<f64>,
}

/// Aggregates individual route results into a unified route network (similar to stplanr::overline)
pub fn aggregate_route_network(
    graph: &StreetGraph,
    routes: &[RouteResult],
) -> Vec<NetworkSegment> {
    // Map edge_id -> (total_students, total_godutch, route_count)
    let mut edge_demand: HashMap<usize, (f64, f64, usize)> = HashMap::new();

    for route in routes {
        for &edge_id in &route.edge_ids {
            let entry = edge_demand.entry(edge_id).or_insert((0.0, 0.0, 0));
            entry.0 += route.num_students;
            entry.1 += route.bicycle_godutch;
            entry.2 += 1;
        }
    }

    let mut network_segments = Vec::new();

    for (edge_id, (students, godutch, count)) in edge_demand {
        if edge_id >= graph.edges.len() {
            continue;
        }
        let edge = &graph.edges[edge_id];

        network_segments.push(NetworkSegment {
            edge_id,
            way_id: edge.way_id,
            name: edge.attributes.name.clone(),
            highway: edge.attributes.highway.clone(),
            length_m: edge.length_m,
            quietness: edge.attributes.quietness_score,
            num_students: students,
            bicycle_godutch: godutch,
            route_count: count,
            geometry: edge.geometry.clone(),
        });
    }

    // Sort by cycling demand descending
    network_segments.sort_by(|a, b| {
        b.bicycle_godutch
            .partial_cmp(&a.bicycle_godutch)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    network_segments
}
