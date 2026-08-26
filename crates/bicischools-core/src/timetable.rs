use crate::bicibus::{CandidateRoute, MatchedOrigin};
use crate::graph::{haversine_distance, StreetGraph};
use geo::Coord;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimetableStop {
    pub stop_id: String,
    pub stop_name: String,
    pub stop_label: String, // "A", "B", "C", ... "Arrival"
    pub lng: f64,
    pub lat: f64,
    pub cumulative_dist_m: f64,
    pub distance_to_next_m: f64,
    pub arrival_time: String,   // "HH:MM"
    pub departure_time: String, // "HH:MM"
    pub boarding_students: f64,
    pub cumulative_students: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteTimetable {
    pub route_rank: usize,
    pub total_distance_m: f64,
    pub total_duration_mins: f64,
    pub average_speed_kmh: f64,
    pub departure_time: String,
    pub arrival_time: String,
    pub stops: Vec<TimetableStop>,
}

/// Helper to extract primary road and intersecting cross-street name near a coordinate
fn get_stop_street_info(graph: &StreetGraph, lng: f64, lat: f64) -> (Option<String>, Option<String>) {
    let nearest_node_idx = match graph.find_nearest_node(lng, lat) {
        Some(idx) => idx,
        None => return (None, None),
    };

    let node = &graph.nodes[nearest_node_idx];
    let dist_to_node = haversine_distance(lng, lat, node.lng, node.lat);

    let mut primary_name: Option<String> = None;
    let mut cross_name: Option<String> = None;

    // Check outgoing edges from this node
    for &edge_idx in &node.outgoing_edges {
        if edge_idx < graph.edges.len() {
            let edge = &graph.edges[edge_idx];
            if let Some(ref name) = edge.attributes.name {
                let clean = name.trim().to_string();
                if !clean.is_empty() {
                    if primary_name.is_none() {
                        primary_name = Some(clean);
                    } else if primary_name.as_ref() != Some(&clean) && cross_name.is_none() {
                        cross_name = Some(clean);
                    }
                }
            }
        }
    }

    // Check incoming edges
    if primary_name.is_none() || cross_name.is_none() {
        for edge in &graph.edges {
            if edge.to_node == nearest_node_idx {
                if let Some(ref name) = edge.attributes.name {
                    let clean = name.trim().to_string();
                    if !clean.is_empty() {
                        if primary_name.is_none() {
                            primary_name = Some(clean);
                        } else if primary_name.as_ref() != Some(&clean) && cross_name.is_none() {
                            cross_name = Some(clean);
                        }
                    }
                }
            }
            if primary_name.is_some() && cross_name.is_some() {
                break;
            }
        }
    }

    // Only treat cross street as junction if sufficiently close to the node
    if dist_to_node > 55.0 {
        cross_name = None;
    }

    (primary_name, cross_name)
}

/// Generates scheduled timetable stops along a candidate bike bus route
pub fn generate_route_timetable(
    route: &CandidateRoute,
    matched_origins: &[MatchedOrigin],
    target_arrival_hhmm: &str, // e.g. "08:45"
    group_speed_kmh: f64,      // e.g. 11.0
    dwell_time_mins: f64,      // e.g. 1.0
    target_stop_spacing_m: f64,// e.g. 350.0
    graph: Option<&StreetGraph>,
    school_name: Option<&str>,
) -> RouteTimetable {
    let speed_mps = (group_speed_kmh * 1000.0) / 3600.0;
    let coords: Vec<Coord<f64>> = route.geometry.coords().cloned().collect();

    if coords.len() < 2 {
        return RouteTimetable {
            route_rank: route.rank,
            total_distance_m: 0.0,
            total_duration_mins: 0.0,
            average_speed_kmh: group_speed_kmh,
            departure_time: target_arrival_hhmm.to_string(),
            arrival_time: target_arrival_hhmm.to_string(),
            stops: Vec::new(),
        };
    }

    // 1. Identify stop coordinates along route
    let mut raw_stops: Vec<(f64, f64, f64)> = Vec::new(); // (lng, lat, cumulative_dist)
    raw_stops.push((coords[0].x, coords[0].y, 0.0));

    let mut current_dist = 0.0;
    let mut last_stop_dist = 0.0;

    for i in 0..coords.len() - 1 {
        let p1 = coords[i];
        let p2 = coords[i + 1];
        let seg_len = haversine_distance(p1.x, p1.y, p2.x, p2.y);

        if current_dist + seg_len - last_stop_dist >= target_stop_spacing_m {
            let needed = target_stop_spacing_m - (current_dist - last_stop_dist);
            let t = (needed / seg_len).clamp(0.1, 0.9);
            let stop_lng = p1.x + t * (p2.x - p1.x);
            let stop_lat = p1.y + t * (p2.y - p1.y);
            let stop_dist = current_dist + needed;

            raw_stops.push((stop_lng, stop_lat, stop_dist));
            last_stop_dist = stop_dist;
        }

        current_dist += seg_len;
    }

    // Final destination stop (School)
    let last_pt = coords[coords.len() - 1];
    raw_stops.push((last_pt.x, last_pt.y, current_dist));

    let total_dist_m = current_dist;

    // 2. Parse target arrival time into seconds from midnight
    let target_arrival_secs = parse_hhmm_to_seconds(target_arrival_hhmm).unwrap_or(8 * 3600 + 45 * 60);

    // 3. Count boarding students at each stop from matched origins
    let mut boarding_counts = vec![0.0; raw_stops.len()];
    let my_origins: Vec<&MatchedOrigin> = matched_origins
        .iter()
        .filter(|o| o.assigned_route_rank == Some(route.rank))
        .collect();

    for origin in my_origins {
        let mut min_stop_dist = f64::INFINITY;
        let mut closest_stop_idx = 0;

        for (idx, &(slng, slat, _)) in raw_stops.iter().enumerate() {
            let d = haversine_distance(origin.lng, origin.lat, slng, slat);
            if d < min_stop_dist {
                min_stop_dist = d;
                closest_stop_idx = idx;
            }
        }
        boarding_counts[closest_stop_idx] += origin.num_students;
    }

    // 4. Back-calculate timings
    let num_stops = raw_stops.len();
    let mut stop_arrivals_secs = vec![0.0; num_stops];
    let mut stop_departures_secs = vec![0.0; num_stops];

    stop_arrivals_secs[num_stops - 1] = target_arrival_secs as f64;
    stop_departures_secs[num_stops - 1] = target_arrival_secs as f64;

    for i in (0..num_stops - 1).rev() {
        let dist_to_next = raw_stops[i + 1].2 - raw_stops[i].2;
        let travel_time_secs = dist_to_next / speed_mps;

        let dep_time = stop_arrivals_secs[i + 1] - travel_time_secs;
        stop_departures_secs[i] = dep_time;

        if i == 0 {
            stop_arrivals_secs[i] = dep_time;
        } else {
            stop_arrivals_secs[i] = dep_time - (dwell_time_mins * 60.0);
        }
    }

    let start_time_secs = stop_departures_secs[0];
    let total_duration_mins = (target_arrival_secs as f64 - start_time_secs) / 60.0;

    let stop_letters = [
        "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R",
        "S", "T", "U", "V", "W", "X", "Y", "Z",
    ];

    let mut timetable_stops = Vec::new();
    let mut cum_students = 0.0;
    let mut street_stop_counts: HashMap<String, usize> = HashMap::new();

    for (i, &(lng, lat, cum_dist)) in raw_stops.iter().enumerate() {
        let is_last = i == num_stops - 1;
        let is_first = i == 0;
        let letter = stop_letters.get(i).copied().unwrap_or("S");
        let label = if is_last {
            "Arr".to_string()
        } else {
            format!("{}{}", route.rank, letter)
        };

        let name = if is_last {
            if let Some(sname) = school_name {
                format!("{sname} (Arrival)")
            } else {
                "School (Arrival)".to_string()
            }
        } else if let Some(g) = graph {
            let (primary_opt, cross_opt) = get_stop_street_info(g, lng, lat);
            match (primary_opt, cross_opt) {
                (Some(primary), Some(cross)) => {
                    if is_first {
                        format!("{primary} near junction with {cross} (Start)")
                    } else {
                        format!("{primary} near junction with {cross}")
                    }
                }
                (Some(primary), None) => {
                    let count = street_stop_counts.entry(primary.clone()).or_insert(0);
                    *count += 1;
                    if is_first {
                        format!("{primary}, stop {count} (Start)")
                    } else {
                        format!("{primary}, stop {count}")
                    }
                }
                (None, _) => {
                    if is_first {
                        format!("Stop {label} (Origin Start)")
                    } else {
                        format!("Stop {label}")
                    }
                }
            }
        } else {
            if is_first {
                format!("Stop {label} (Origin Start)")
            } else {
                format!("Stop {label}")
            }
        };

        let dist_to_next = if is_last {
            0.0
        } else {
            raw_stops[i + 1].2 - cum_dist
        };

        cum_students += boarding_counts[i];

        timetable_stops.push(TimetableStop {
            stop_id: format!("R{}_{}", route.rank, label),
            stop_name: name,
            stop_label: label,
            lng,
            lat,
            cumulative_dist_m: cum_dist,
            distance_to_next_m: dist_to_next,
            arrival_time: seconds_to_hhmm(stop_arrivals_secs[i]),
            departure_time: seconds_to_hhmm(stop_departures_secs[i]),
            boarding_students: boarding_counts[i],
            cumulative_students: cum_students,
        });
    }

    RouteTimetable {
        route_rank: route.rank,
        total_distance_m: total_dist_m,
        total_duration_mins,
        average_speed_kmh: group_speed_kmh,
        departure_time: seconds_to_hhmm(start_time_secs),
        arrival_time: target_arrival_hhmm.to_string(),
        stops: timetable_stops,
    }
}

fn parse_hhmm_to_seconds(hhmm: &str) -> Option<u32> {
    let parts: Vec<&str> = hhmm.trim().split(':').collect();
    if parts.len() == 2 {
        let h: u32 = parts[0].parse().ok()?;
        let m: u32 = parts[1].parse().ok()?;
        Some(h * 3600 + m * 60)
    } else {
        None
    }
}

fn seconds_to_hhmm(secs: f64) -> String {
    let total_mins = (secs / 60.0).round() as i64;
    let positive_mins = (total_mins % (24 * 60) + (24 * 60)) % (24 * 60);
    let h = positive_mins / 60;
    let m = positive_mins % 60;
    format!("{h:02}:{m:02}")
}
