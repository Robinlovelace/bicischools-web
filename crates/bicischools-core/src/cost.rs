use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Routing profile type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RoutingProfile {
    Quiet,
    Fast,
}

impl Default for RoutingProfile {
    fn default() -> Self {
        Self::Quiet
    }
}

/// Road/path characteristics derived from OSM tags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeAttributes {
    pub highway: String,
    pub name: Option<String>,
    pub maxspeed_kmh: Option<f64>,
    pub is_cycleway: bool,
    pub has_cycle_infra: bool,
    pub is_one_way: bool,
    pub bicycle_allowed: bool,
    pub surface: Option<String>,
    pub quietness_score: f64, // 0 to 100
    pub elevation_change_m: f64,
    pub gradient_ratio: f64,
}

impl EdgeAttributes {
    pub fn from_tags(tags: &HashMap<String, String>, _length_m: f64) -> Self {
        let highway = tags.get("highway").cloned().unwrap_or_default();
        let name = tags.get("name").cloned();
        let bicycle = tags.get("bicycle").map(|s| s.as_str());
        let cycleway = tags.get("cycleway").map(|s| s.as_str());
        let cycleway_left = tags.get("cycleway:left").map(|s| s.as_str());
        let cycleway_right = tags.get("cycleway:right").map(|s| s.as_str());
        let cycleway_both = tags.get("cycleway:both").map(|s| s.as_str());
        let surface = tags.get("surface").cloned();
        let oneway = tags.get("oneway").map(|s| s.as_str());
        let oneway_bicycle = tags.get("oneway:bicycle").map(|s| s.as_str());

        let maxspeed_kmh = tags.get("maxspeed").and_then(|s| {
            s.split_whitespace()
                .next()
                .and_then(|val| val.parse::<f64>().ok())
        });

        let is_cycleway = highway == "cycleway" || bicycle == Some("designated");

        let has_cycle_infra = is_cycleway
            || cycleway.is_some()
            || cycleway_left.is_some()
            || cycleway_right.is_some()
            || cycleway_both.is_some();

        let bicycle_allowed = match bicycle {
            Some("no") => false,
            Some("dismount") => true,
            _ => match highway.as_str() {
                "motorway" | "motorway_link" | "trunk" | "trunk_link" => has_cycle_infra,
                "corridor" | "steps" | "construction" | "proposed" => false,
                "footway" | "pedestrian" => {
                    bicycle == Some("yes") || bicycle == Some("designated") || bicycle == Some("permissive")
                }
                _ => true,
            },
        };

        let is_one_way = match oneway {
            Some("yes") | Some("1") | Some("true") => oneway_bicycle != Some("no"),
            Some("-1") => true,
            _ => false,
        };

        let quietness_score = calculate_quietness(
            &highway,
            has_cycle_infra,
            bicycle_allowed,
            maxspeed_kmh,
            surface.as_deref(),
        );

        Self {
            highway,
            name,
            maxspeed_kmh,
            is_cycleway,
            has_cycle_infra,
            is_one_way,
            bicycle_allowed,
            surface,
            quietness_score,
            elevation_change_m: 0.0,
            gradient_ratio: 0.0,
        }
    }

    /// Calculate edge traversal cost (weight) based on routing profile
    pub fn calculate_cost(&self, length_m: f64, profile: RoutingProfile) -> f64 {
        if !self.bicycle_allowed {
            return f64::INFINITY;
        }

        match profile {
            RoutingProfile::Fast => {
                let speed_kmh = self.maxspeed_kmh.unwrap_or(20.0).clamp(10.0, 25.0);
                let time_sec = (length_m / (speed_kmh * 1000.0 / 3600.0)).max(0.1);
                length_m + time_sec * 2.0
            }
            RoutingProfile::Quiet => {
                let quietness_factor = (100.0 - self.quietness_score).max(0.0) / 100.0;
                let multiplier = 0.7 + (quietness_factor * 3.5).powf(1.8);

                let grade_penalty = if self.gradient_ratio > 0.02 {
                    1.0 + (self.gradient_ratio * 15.0).powi(2)
                } else {
                    1.0
                };

                length_m * multiplier * grade_penalty
            }
        }
    }
}

/// Computes a cycle quietness score from 0 (very high stress) to 100 (dedicated, safe cycle infrastructure)
fn calculate_quietness(
    highway: &str,
    has_cycle_infra: bool,
    bicycle_allowed: bool,
    maxspeed: Option<f64>,
    surface: Option<&str>,
) -> f64 {
    if !bicycle_allowed {
        return 0.0;
    }

    let mut base_score: f64 = match highway {
        "cycleway" => 100.0,
        "living_street" => 95.0,
        "pedestrian" => 90.0,
        "path" => 88.0,
        "residential" => 80.0,
        "unclassified" => 75.0,
        "service" => 70.0,
        "tertiary" | "tertiary_link" => 50.0,
        "secondary" | "secondary_link" => 25.0,
        "primary" | "primary_link" => 10.0,
        "trunk" | "trunk_link" => 5.0,
        "track" => 65.0,
        _ => 40.0,
    };

    if has_cycle_infra && highway != "cycleway" {
        base_score = (base_score + 40.0).min(95.0);
    }

    if let Some(spd) = maxspeed {
        if spd <= 20.0 {
            base_score = (base_score + 10.0).min(100.0);
        } else if spd <= 30.0 {
            base_score = (base_score + 5.0).min(100.0);
        } else if spd >= 60.0 {
            base_score = (base_score - 25.0).max(0.0);
        } else if spd >= 50.0 {
            base_score = (base_score - 15.0).max(0.0);
        }
    }

    if let Some(surf) = surface {
        match surf {
            "cobblestone" | "sett" | "unpaved" | "gravel" | "dirt" | "sand" => {
                base_score = (base_score - 10.0).max(10.0);
            }
            "asphalt" | "paved" | "concrete" => {
                base_score = (base_score + 5.0).min(100.0);
            }
            _ => {}
        }
    }

    base_score.clamp(0.0, 100.0)
}
