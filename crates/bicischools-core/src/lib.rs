pub mod bicibus;
pub mod cost;
pub mod graph;
pub mod osm;
pub mod overline;
pub mod router;
pub mod synthetic;
pub mod timetable;
pub mod uptake;

pub use bicibus::{generate_candidate_bike_buses, CandidateRoute, MatchedOrigin, PlanningSummary};
pub use cost::{EdgeAttributes, RoutingProfile};
pub use graph::{haversine_distance, Edge, Node, StreetGraph};
pub use osm::{parse_overpass_json, OverpassResponse};
pub use overline::{aggregate_route_network, NetworkSegment};
pub use router::{route_all_origins_to_school, RouteResult};
pub use synthetic::generate_synthetic_origins;
pub use timetable::{generate_route_timetable, RouteTimetable, TimetableStop};
pub use uptake::calculate_go_dutch_school;

use serde::{Deserialize, Serialize};

/// Origin point input specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OriginInput {
    pub id: String,
    pub lng: f64,
    pub lat: f64,
    pub num_students: f64,
}

/// Analysis Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiciConfig {
    pub school_lng: f64,
    pub school_lat: f64,
    pub school_name: Option<String>,
    #[serde(default)]
    pub origins: Vec<OriginInput>,
    #[serde(default)]
    pub routing_profile: RoutingProfile,
    #[serde(default = "default_min_trips")]
    pub min_trips_threshold: f64,
    #[serde(default = "default_origin_buffer")]
    pub origin_buffer_m: f64,
    #[serde(default = "default_max_routes")]
    pub max_routes: usize,
    #[serde(default = "default_max_dist_to_bikebus")]
    pub max_dist_to_bikebus_m: f64,
    #[serde(default = "default_arrival_time")]
    pub target_arrival_time: String,
    #[serde(default = "default_group_speed")]
    pub group_speed_kmh: f64,
    #[serde(default = "default_dwell_time")]
    pub dwell_time_mins: f64,
    #[serde(default = "default_max_route_distance")]
    pub max_route_distance_m: f64,
    #[serde(default = "default_circuity")]
    pub circuity: f64, // 1.0 = direct/shortest, 1.25 = moderate meandering, 2.0 = high circuity
}

fn default_min_trips() -> f64 { 3.0 }
fn default_origin_buffer() -> f64 { 300.0 }
fn default_max_routes() -> usize { 3 }
fn default_max_dist_to_bikebus() -> f64 { 300.0 }
fn default_arrival_time() -> String { "08:45".to_string() }
fn default_group_speed() -> f64 { 11.0 }
fn default_dwell_time() -> f64 { 1.0 }
fn default_max_route_distance() -> f64 { 5000.0 }
fn default_circuity() -> f64 { 1.25 }

impl Default for BiciConfig {
    fn default() -> Self {
        Self {
            school_lng: 0.0,
            school_lat: 0.0,
            school_name: None,
            origins: Vec::new(),
            routing_profile: RoutingProfile::Quiet,
            min_trips_threshold: default_min_trips(),
            origin_buffer_m: default_origin_buffer(),
            max_routes: default_max_routes(),
            max_dist_to_bikebus_m: default_max_dist_to_bikebus(),
            target_arrival_time: default_arrival_time(),
            group_speed_kmh: default_group_speed(),
            dwell_time_mins: default_dwell_time(),
            max_route_distance_m: default_max_route_distance(),
            circuity: default_circuity(),
        }
    }
}

/// Analysis output containing all generated spatial layers and timetables
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiciAnalysisResult {
    pub candidate_routes: Vec<CandidateRoute>,
    pub route_network: Vec<NetworkSegment>,
    pub matched_origins: Vec<MatchedOrigin>,
    pub timetables: Vec<RouteTimetable>,
    pub summary: PlanningSummary,
    pub routes_count: usize,
}

/// Main BiciEngine instance
pub struct BiciEngine {
    pub graph: StreetGraph,
}

impl BiciEngine {
    pub fn from_overpass_json(osm_json: &str) -> anyhow::Result<Self> {
        let overpass = parse_overpass_json(osm_json)?;
        let graph = StreetGraph::from_overpass(&overpass);
        Ok(Self { graph })
    }

    /// Runs end-to-end bike bus planning workflow
    pub fn run_analysis(&self, config: &BiciConfig) -> anyhow::Result<BiciAnalysisResult> {
        // 1. Locate school destination node
        let dest_node = self
            .graph
            .find_nearest_node(config.school_lng, config.school_lat)
            .ok_or_else(|| anyhow::anyhow!("No road network nodes found near school location"))?;

        // 2. Prepare origins: use user-provided origins or generate synthetic ones
        let origins_to_route: Vec<(String, usize, f64)> = if !config.origins.is_empty() {
            config
                .origins
                .iter()
                .filter_map(|o| {
                    self.graph
                        .find_nearest_node(o.lng, o.lat)
                        .map(|nid| (o.id.clone(), nid, o.num_students.max(0.1)))
                })
                .collect()
        } else {
            generate_synthetic_origins(
                &self.graph,
                config.school_lng,
                config.school_lat,
                120,
                config.max_route_distance_m.min(3500.0),
                42,
            )
        };

        if origins_to_route.is_empty() {
            anyhow::bail!("No valid origin nodes found within catchment area");
        }

        // 3. Route all origins to school with circuity parameter
        let individual_routes = route_all_origins_to_school(
            &self.graph,
            dest_node,
            &origins_to_route,
            config.routing_profile,
            config.max_route_distance_m,
            config.circuity,
        );

        if individual_routes.is_empty() {
            anyhow::bail!("Could not compute routes to school. Ensure road network is fully connected.");
        }

        // 4. Aggregate route network (stplanr::overline)
        let route_network = aggregate_route_network(&self.graph, &individual_routes);

        // 5. Generate candidate bike bus routes, ranking, and student matching
        let (candidate_routes, matched_origins, summary) = generate_candidate_bike_buses(
            &self.graph,
            &individual_routes,
            &route_network,
            config.min_trips_threshold,
            config.origin_buffer_m,
            config.max_routes,
            config.max_dist_to_bikebus_m,
        );

        // 6. Generate timetables for each candidate route
        let timetables: Vec<RouteTimetable> = candidate_routes
            .iter()
            .map(|cr| {
                generate_route_timetable(
                    cr,
                    &matched_origins,
                    &config.target_arrival_time,
                    config.group_speed_kmh,
                    config.dwell_time_mins,
                    350.0,
                )
            })
            .collect();

        Ok(BiciAnalysisResult {
            routes_count: individual_routes.len(),
            candidate_routes,
            route_network,
            matched_origins,
            timetables,
            summary,
        })
    }
}
