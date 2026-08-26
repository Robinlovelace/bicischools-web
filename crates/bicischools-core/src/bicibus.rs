use crate::graph::{haversine_distance, StreetGraph};
use crate::overline::NetworkSegment;
use crate::router::RouteResult;
use geo::{Coord, LineString};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Candidate Bike Bus corridor route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateRoute {
    pub id: usize,
    pub rank: usize,
    pub origin_id: String,
    pub start_lng: f64,
    pub start_lat: f64,
    pub total_length_m: f64,
    pub corridor_length_m: f64,
    pub score: f64,
    pub mean_godutch_demand: f64,
    pub quietness_score: f64,
    pub accommodated_students: f64,
    pub accommodated_godutch: f64,
    pub geometry: LineString<f64>,
}

/// Matched origin information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedOrigin {
    pub origin_id: String,
    pub lng: f64,
    pub lat: f64,
    pub num_students: f64,
    pub pcycle_godutch: f64,
    pub bicycle_godutch: f64,
    pub assigned_route_rank: Option<usize>,
    pub dist_to_bike_bus_m: f64,
    pub bike_bus_length_m: f64,
    pub total_route_length_m: f64,
}

/// Overall Bike Bus Planning Analysis Summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningSummary {
    pub total_origins: usize,
    pub total_students: f64,
    pub total_godutch_potential: f64,
    pub accommodated_students: f64,
    pub accommodated_students_pct: f64,
    pub accommodated_godutch: f64,
    pub accommodated_godutch_pct: f64,
    pub median_dist_to_bike_bus_m: f64,
    pub mean_dist_to_bike_bus_m: f64,
    pub total_network_km: f64,
}

/// Distance from a point to a LineString in meters
pub fn point_to_linestring_min_dist_m(lng: f64, lat: f64, line: &LineString<f64>) -> f64 {
    let mut min_d = f64::INFINITY;
    for segment in line.lines() {
        let d = dist_point_to_segment_m(lng, lat, segment.start.x, segment.start.y, segment.end.x, segment.end.y);
        if d < min_d {
            min_d = d;
        }
    }
    min_d
}

/// Approximate distance from point (px, py) to line segment (x1, y1)-(x2, y2) in meters
pub fn dist_point_to_segment_m(px: f64, py: f64, x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len_sq = dx * dx + dy * dy;

    if len_sq < 1e-12 {
        return haversine_distance(px, py, x1, y1);
    }

    let t = (((px - x1) * dx + (py - y1) * dy) / len_sq).clamp(0.0, 1.0);
    let proj_x = x1 + t * dx;
    let proj_y = y1 + t * dy;

    haversine_distance(px, py, proj_x, proj_y)
}

/// Identifies and prioritizes candidate bike bus routes
pub fn generate_candidate_bike_buses(
    graph: &StreetGraph,
    routes: &[RouteResult],
    network: &[NetworkSegment],
    min_trips_threshold: f64,
    origin_buffer_m: f64,
    max_routes: usize,
    max_dist_to_bikebus_m: f64,
) -> (Vec<CandidateRoute>, Vec<MatchedOrigin>, PlanningSummary) {
    if routes.is_empty() {
        return (
            Vec::new(),
            Vec::new(),
            PlanningSummary {
                total_origins: 0,
                total_students: 0.0,
                total_godutch_potential: 0.0,
                accommodated_students: 0.0,
                accommodated_students_pct: 0.0,
                accommodated_godutch: 0.0,
                accommodated_godutch_pct: 0.0,
                median_dist_to_bike_bus_m: 0.0,
                mean_dist_to_bike_bus_m: 0.0,
                total_network_km: 0.0,
            },
        );
    }

    // 1. Map corridor edges with demand >= min_trips_threshold
    let mut corridor_edges: HashMap<usize, f64> = HashMap::new();
    for seg in network {
        if seg.bicycle_godutch >= min_trips_threshold {
            corridor_edges.insert(seg.edge_id, seg.bicycle_godutch);
        }
    }

    // 2. Score candidate routes
    struct ScoredRoute<'a> {
        route: &'a RouteResult,
        corridor_len_m: f64,
        mean_demand: f64,
        score: f64,
    }

    let mut scored_routes = Vec::new();

    for route in routes {
        let mut corridor_len_m = 0.0;
        let mut weighted_demand_sum = 0.0;

        for &edge_id in &route.edge_ids {
            if let Some(&demand) = corridor_edges.get(&edge_id) {
                if edge_id < graph.edges.len() {
                    let len = graph.edges[edge_id].length_m;
                    corridor_len_m += len;
                    weighted_demand_sum += demand * len;
                }
            }
        }

        let mean_demand = if corridor_len_m > 0.0 {
            weighted_demand_sum / corridor_len_m
        } else {
            0.0
        };

        let score = corridor_len_m * mean_demand;

        scored_routes.push(ScoredRoute {
            route,
            corridor_len_m,
            mean_demand,
            score,
        });
    }

    scored_routes.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    // 3. Greedily select top routes with origin buffer separation
    let mut selected: Vec<&ScoredRoute> = Vec::new();

    for candidate in &scored_routes {
        if candidate.score <= 0.0 && !selected.is_empty() {
            continue;
        }

        let start_coord = match candidate.route.geometry.coords().next() {
            Some(c) => c,
            None => continue,
        };

        let mut too_close = false;
        for existing in &selected {
            if let Some(c2) = existing.route.geometry.coords().next() {
                let dist = haversine_distance(start_coord.x, start_coord.y, c2.x, c2.y);
                if dist < origin_buffer_m {
                    too_close = true;
                    break;
                }
            }
        }

        if !too_close {
            selected.push(candidate);
            if selected.len() >= max_routes {
                break;
            }
        }
    }

    // 4. Format CandidateRoutes
    let mut candidate_routes = Vec::new();
    for (i, sr) in selected.iter().enumerate() {
        let rank = i + 1;
        let start_c = sr.route.geometry.coords().next().cloned().unwrap_or(Coord { x: 0.0, y: 0.0 });

        candidate_routes.push(CandidateRoute {
            id: rank,
            rank,
            origin_id: sr.route.origin_id.clone(),
            start_lng: start_c.x,
            start_lat: start_c.y,
            total_length_m: sr.route.length_m,
            corridor_length_m: sr.corridor_len_m,
            score: sr.score,
            mean_godutch_demand: sr.mean_demand,
            quietness_score: sr.route.quietness_score,
            accommodated_students: 0.0,
            accommodated_godutch: 0.0,
            geometry: sr.route.geometry.clone(),
        });
    }

    // 5. Match student origins to nearest candidate route
    let mut matched_origins = Vec::new();
    let mut total_students = 0.0;
    let mut total_godutch_potential = 0.0;
    let mut accommodated_students = 0.0;
    let mut accommodated_godutch = 0.0;
    let mut distances_to_bus = Vec::new();

    for route in routes {
        total_students += route.num_students;
        total_godutch_potential += route.bicycle_godutch;

        let origin_pt = match route.geometry.coords().next() {
            Some(c) => c,
            None => continue,
        };

        let mut min_dist = f64::INFINITY;
        let mut closest_rank = None;

        for cr in &candidate_routes {
            let dist = point_to_linestring_min_dist_m(origin_pt.x, origin_pt.y, &cr.geometry);
            if dist < min_dist {
                min_dist = dist;
                closest_rank = Some(cr.rank);
            }
        }

        let is_accommodated = min_dist <= max_dist_to_bikebus_m;
        let assigned_rank = if is_accommodated { closest_rank } else { None };

        if let Some(rank) = assigned_rank {
            if let Some(cr) = candidate_routes.iter_mut().find(|c| c.rank == rank) {
                cr.accommodated_students += route.num_students;
                cr.accommodated_godutch += route.bicycle_godutch;
            }
            accommodated_students += route.num_students;
            accommodated_godutch += route.bicycle_godutch;
            distances_to_bus.push(min_dist);
        }

        let bike_bus_length = if is_accommodated {
            (route.length_m - min_dist).max(0.0)
        } else {
            0.0
        };

        matched_origins.push(MatchedOrigin {
            origin_id: route.origin_id.clone(),
            lng: origin_pt.x,
            lat: origin_pt.y,
            num_students: route.num_students,
            pcycle_godutch: route.pcycle_godutch,
            bicycle_godutch: route.bicycle_godutch,
            assigned_route_rank: assigned_rank,
            dist_to_bike_bus_m: min_dist,
            bike_bus_length_m: bike_bus_length,
            total_route_length_m: route.length_m,
        });
    }

    distances_to_bus.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_dist = if !distances_to_bus.is_empty() {
        distances_to_bus[distances_to_bus.len() / 2]
    } else {
        0.0
    };
    let mean_dist = if !distances_to_bus.is_empty() {
        distances_to_bus.iter().sum::<f64>() / distances_to_bus.len() as f64
    } else {
        0.0
    };

    let total_network_km = candidate_routes.iter().map(|c| c.total_length_m).sum::<f64>() / 1000.0;

    let summary = PlanningSummary {
        total_origins: routes.len(),
        total_students,
        total_godutch_potential,
        accommodated_students,
        accommodated_students_pct: if total_students > 0.0 {
            (accommodated_students / total_students) * 100.0
        } else {
            0.0
        },
        accommodated_godutch,
        accommodated_godutch_pct: if total_godutch_potential > 0.0 {
            (accommodated_godutch / total_godutch_potential) * 100.0
        } else {
            0.0
        },
        median_dist_to_bike_bus_m: median_dist,
        mean_dist_to_bike_bus_m: mean_dist,
        total_network_km,
    };

    (candidate_routes, matched_origins, summary)
}
