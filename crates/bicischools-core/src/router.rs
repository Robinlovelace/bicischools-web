use crate::cost::RoutingProfile;
use crate::graph::{haversine_distance, StreetGraph};
use crate::uptake::calculate_go_dutch_school;
use geo::{Coord, LineString};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteResult {
    pub origin_id: String,
    pub school_id: String,
    pub num_students: f64,
    pub length_m: f64,
    pub quietness_score: f64,
    pub gradient_pct: f64,
    pub pcycle_godutch: f64,
    pub bicycle_godutch: f64,
    pub edge_ids: Vec<usize>,
    pub geometry: LineString<f64>,
}

#[derive(Copy, Clone, PartialEq)]
struct State {
    cost: f64,
    node: usize,
}

impl Eq for State {}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        other.cost.partial_cmp(&self.cost).unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Computes student density at each graph node to attract circuitous/meandering routes
fn compute_node_student_densities(
    graph: &StreetGraph,
    origins: &[(String, usize, f64)],
) -> Vec<f64> {
    let mut density = vec![0.0; graph.nodes.len()];
    let search_radius_m = 350.0;

    for &(_, origin_node, num_students) in origins {
        if origin_node >= graph.nodes.len() {
            continue;
        }
        let o_node = &graph.nodes[origin_node];

        // Find surrounding nodes within radius
        for (idx, node) in graph.nodes.iter().enumerate() {
            let d = haversine_distance(o_node.lng, o_node.lat, node.lng, node.lat);
            if d <= search_radius_m {
                let weight = (-d / 120.0).exp();
                density[idx] += num_students * weight;
            }
        }
    }

    // Normalize density to [0.0, 1.0]
    let max_density = density.iter().cloned().fold(0.0, f64::max);
    if max_density > 0.0 {
        for val in &mut density {
            *val /= max_density;
        }
    }

    density
}

/// Computes routes from multiple origins to a single destination school with a circuity/meandering parameter
pub fn route_all_origins_to_school(
    graph: &StreetGraph,
    dest_node: usize,
    origin_nodes_and_students: &[(String, usize, f64)],
    profile: RoutingProfile,
    max_distance_m: f64,
    circuity: f64, // 1.0 = direct/shortest, 1.3 = moderate meandering, 2.0 = high circuity
) -> Vec<RouteResult> {
    if graph.nodes.is_empty() || dest_node >= graph.nodes.len() {
        return Vec::new();
    }

    // Pre-calculate node densities if circuity > 1.0
    let node_densities = if circuity > 1.001 {
        compute_node_student_densities(graph, origin_nodes_and_students)
    } else {
        Vec::new()
    };

    let mut reverse_incoming: Vec<Vec<usize>> = vec![Vec::new(); graph.nodes.len()];
    for edge in &graph.edges {
        reverse_incoming[edge.to_node].push(edge.id);
    }

    let mut dist = vec![f64::INFINITY; graph.nodes.len()];
    let mut prev_edge = vec![None; graph.nodes.len()];
    let mut heap = BinaryHeap::new();

    dist[dest_node] = 0.0;
    heap.push(State {
        cost: 0.0,
        node: dest_node,
    });

    let circuity_attraction = (circuity - 1.0).max(0.0) * 3.5;

    while let Some(State { cost, node }) = heap.pop() {
        if cost > dist[node] {
            continue;
        }

        for &edge_id in &reverse_incoming[node] {
            let edge = &graph.edges[edge_id];
            let u = edge.from_node;
            let mut edge_cost = edge.attributes.calculate_cost(edge.length_m, profile);

            // If circuity > 1.0, discount edges with high student density
            // encouraging routes to detour through residential neighborhoods to collect students
            if circuity_attraction > 0.0 && !node_densities.is_empty() {
                let d_u = node_densities[u];
                let d_v = node_densities[node];
                let edge_density = (d_u + d_v) * 0.5;

                let discount = 1.0 / (1.0 + circuity_attraction * edge_density);
                edge_cost *= discount;
            }

            if edge_cost.is_finite() && dist[node] + edge_cost < dist[u] {
                dist[u] = dist[node] + edge_cost;
                prev_edge[u] = Some(edge_id);
                heap.push(State {
                    cost: dist[u],
                    node: u,
                });
            }
        }
    }

    let mut results = Vec::new();

    for (origin_id, start_node, num_students) in origin_nodes_and_students {
        if *start_node >= graph.nodes.len() || dist[*start_node].is_infinite() {
            continue;
        }

        let mut curr = *start_node;
        let mut edge_ids = Vec::new();
        let mut coords = Vec::new();

        coords.push(Coord {
            x: graph.nodes[curr].lng,
            y: graph.nodes[curr].lat,
        });

        let mut total_length_m = 0.0;
        let mut weighted_quietness_sum = 0.0;
        let mut loop_detector = 0;
        let max_steps = graph.nodes.len();

        while curr != dest_node && loop_detector < max_steps {
            loop_detector += 1;
            match prev_edge[curr] {
                Some(edge_idx) => {
                    let edge = &graph.edges[edge_idx];
                    edge_ids.push(edge_idx);
                    total_length_m += edge.length_m;
                    weighted_quietness_sum += edge.attributes.quietness_score * edge.length_m;

                    for c in edge.geometry.coords().skip(1) {
                        coords.push(*c);
                    }
                    curr = edge.to_node;
                }
                None => break,
            }
        }

        if curr != dest_node || total_length_m > (max_distance_m * circuity) || total_length_m <= 0.0 {
            continue;
        }

        let quietness_score = if total_length_m > 0.0 {
            weighted_quietness_sum / total_length_m
        } else {
            50.0
        };

        let gradient_pct = 1.0;
        let pcycle_godutch = calculate_go_dutch_school(total_length_m, gradient_pct);
        let bicycle_godutch = (*num_students) * pcycle_godutch;

        results.push(RouteResult {
            origin_id: origin_id.clone(),
            school_id: format!("school_{dest_node}"),
            num_students: *num_students,
            length_m: total_length_m,
            quietness_score,
            gradient_pct,
            pcycle_godutch,
            bicycle_godutch,
            edge_ids,
            geometry: LineString::new(coords),
        });
    }

    results
}
