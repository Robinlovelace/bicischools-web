use crate::graph::{haversine_distance, StreetGraph};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntheticOrigin {
    pub id: String,
    pub lng: f64,
    pub lat: f64,
    pub num_students: f64,
    pub distance_to_school_m: f64,
}

/// Generates synthetic student origins sampled from residential street nodes around a school
pub fn generate_synthetic_origins(
    graph: &StreetGraph,
    school_lng: f64,
    school_lat: f64,
    target_student_count: usize,
    max_radius_m: f64,
    seed: u64,
) -> Vec<(String, usize, f64)> {
    let mut rng = StdRng::seed_from_u64(seed);

    // 1. Collect eligible residential / minor street nodes within radius
    let mut eligible_nodes = Vec::new();

    for node in &graph.nodes {
        let dist = haversine_distance(school_lng, school_lat, node.lng, node.lat);
        if dist > 100.0 && dist <= max_radius_m {
            // Check if connected to residential / path / quiet roads
            let is_residential = node.outgoing_edges.iter().any(|&eid| {
                if eid < graph.edges.len() {
                    let hw = &graph.edges[eid].attributes.highway;
                    hw == "residential"
                        || hw == "living_street"
                        || hw == "unclassified"
                        || hw == "tertiary"
                        || hw == "cycleway"
                        || hw == "path"
                } else {
                    false
                }
            });

            if is_residential {
                // Weight by distance (distance decay curve peaking at 500m - 1800m)
                let dist_km = dist / 1000.0;
                let weight = (dist_km * (-dist_km * 0.8).exp()).max(0.01);
                eligible_nodes.push((node.id, dist, weight));
            }
        }
    }

    if eligible_nodes.is_empty() {
        return Vec::new();
    }

    // 2. Sample target_student_count origins
    let total_weight: f64 = eligible_nodes.iter().map(|(_, _, w)| *w).sum();
    let mut sampled_origins = Vec::new();
    let num_clusters = (target_student_count / 3).max(10).min(eligible_nodes.len());

    for i in 0..num_clusters {
        let mut r = rng.gen::<f64>() * total_weight;
        let mut chosen_node = eligible_nodes[0].0;

        for &(nid, _, w) in &eligible_nodes {
            if r <= w {
                chosen_node = nid;
                break;
            }
            r -= w;
        }

        let students_in_cluster = rng.gen_range(1..=4) as f64;
        sampled_origins.push((format!("synthetic_{}", i + 1), chosen_node, students_in_cluster));
    }

    sampled_origins
}
