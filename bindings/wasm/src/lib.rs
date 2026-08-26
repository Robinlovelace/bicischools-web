use bicischools_core::{
    generate_route_timetable, generate_synthetic_origins, BiciConfig, BiciEngine, CandidateRoute,
    MatchedOrigin, OriginInput, RouteTimetable,
};
use geojson::{Feature, FeatureCollection, Geometry, JsonObject, Value};
use serde::{Deserialize, Serialize};
use std::sync::Once;
use wasm_bindgen::prelude::*;

static INIT_LOG: Once = Once::new();

#[wasm_bindgen(start)]
pub fn init() {
    INIT_LOG.call_once(|| {
        console_error_panic_hook::set_once();
        let _ = console_log::init_with_level(log::Level::Info);
    });
}

#[wasm_bindgen]
pub struct WasmBiciEngine {
    engine: BiciEngine,
}

#[wasm_bindgen]
impl WasmBiciEngine {
    /// Creates a new engine from Overpass OSM JSON text
    #[wasm_bindgen(constructor)]
    pub fn new(osm_json: &str) -> Result<WasmBiciEngine, JsValue> {
        let engine = BiciEngine::from_overpass_json(osm_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse OSM data: {e}")))?;
        Ok(WasmBiciEngine { engine })
    }

    /// Runs the bike bus route identification and planning algorithm
    #[wasm_bindgen(js_name = runAnalysis)]
    pub fn run_analysis(&self, config_json: &str) -> Result<String, JsValue> {
        let config: BiciConfig = serde_json::from_str(config_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid configuration JSON: {e}")))?;

        let result = self
            .engine
            .run_analysis(&config)
            .map_err(|e| JsValue::from_str(&format!("Analysis failed: {e}")))?;

        // Format into GeoJSON collections
        let candidate_routes_fc = format_candidate_routes_geojson(&result.candidate_routes);
        let route_network_fc = format_route_network_geojson(&result.route_network);
        let matched_origins_fc = format_matched_origins_geojson(&result.matched_origins);

        let output = serde_json::json!({
            "routes_count": result.routes_count,
            "candidate_routes": result.candidate_routes,
            "candidate_routes_geojson": candidate_routes_fc,
            "route_network_geojson": route_network_fc,
            "matched_origins_geojson": matched_origins_fc,
            "timetables": result.timetables,
            "summary": result.summary
        });

        serde_json::to_string(&output)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {e}")))
    }

    /// Generates synthetic origins around a school
    #[wasm_bindgen(js_name = generateSyntheticOrigins)]
    pub fn generate_synthetic_origins(
        &self,
        school_lng: f64,
        school_lat: f64,
        count: usize,
        radius_m: f64,
    ) -> Result<String, JsValue> {
        let origins = generate_synthetic_origins(
            &self.engine.graph,
            school_lng,
            school_lat,
            count,
            radius_m,
            42,
        );

        let inputs: Vec<OriginInput> = origins
            .into_iter()
            .map(|(id, nid, students)| {
                let node = &self.engine.graph.nodes[nid];
                OriginInput {
                    id,
                    lng: node.lng,
                    lat: node.lat,
                    num_students: students,
                }
            })
            .collect();

        serde_json::to_string(&inputs)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {e}")))
    }
}

fn format_candidate_routes_geojson(routes: &[CandidateRoute]) -> FeatureCollection {
    let features: Vec<Feature> = routes
        .iter()
        .map(|r| {
            let coords: Vec<Vec<f64>> = r
                .geometry
                .coords()
                .map(|c| vec![c.x, c.y])
                .collect();

            let geom = Geometry::new(Value::LineString(coords));
            let mut props = JsonObject::new();
            props.insert("id".to_string(), serde_json::json!(r.id));
            props.insert("rank".to_string(), serde_json::json!(r.rank));
            props.insert("origin_id".to_string(), serde_json::json!(r.origin_id));
            props.insert("total_length_m".to_string(), serde_json::json!(r.total_length_m));
            props.insert("corridor_length_m".to_string(), serde_json::json!(r.corridor_length_m));
            props.insert("score".to_string(), serde_json::json!(r.score));
            props.insert("mean_godutch_demand".to_string(), serde_json::json!(r.mean_godutch_demand));
            props.insert("quietness_score".to_string(), serde_json::json!(r.quietness_score));
            props.insert("accommodated_students".to_string(), serde_json::json!(r.accommodated_students));
            props.insert("accommodated_godutch".to_string(), serde_json::json!(r.accommodated_godutch));

            Feature {
                bbox: None,
                geometry: Some(geom),
                id: None,
                properties: Some(props),
                foreign_members: None,
            }
        })
        .collect();

    FeatureCollection {
        bbox: None,
        features,
        foreign_members: None,
    }
}

fn format_route_network_geojson(network: &[bicischools_core::NetworkSegment]) -> FeatureCollection {
    let features: Vec<Feature> = network
        .iter()
        .map(|seg| {
            let coords: Vec<Vec<f64>> = seg
                .geometry
                .coords()
                .map(|c| vec![c.x, c.y])
                .collect();

            let geom = Geometry::new(Value::LineString(coords));
            let mut props = JsonObject::new();
            props.insert("edge_id".to_string(), serde_json::json!(seg.edge_id));
            props.insert("name".to_string(), serde_json::json!(seg.name));
            props.insert("highway".to_string(), serde_json::json!(seg.highway));
            props.insert("length_m".to_string(), serde_json::json!(seg.length_m));
            props.insert("quietness".to_string(), serde_json::json!(seg.quietness));
            props.insert("num_students".to_string(), serde_json::json!(seg.num_students));
            props.insert("bicycle_godutch".to_string(), serde_json::json!(seg.bicycle_godutch));
            props.insert("route_count".to_string(), serde_json::json!(seg.route_count));

            Feature {
                bbox: None,
                geometry: Some(geom),
                id: None,
                properties: Some(props),
                foreign_members: None,
            }
        })
        .collect();

    FeatureCollection {
        bbox: None,
        features,
        foreign_members: None,
    }
}

fn format_matched_origins_geojson(origins: &[MatchedOrigin]) -> FeatureCollection {
    let features: Vec<Feature> = origins
        .iter()
        .map(|o| {
            let geom = Geometry::new(Value::Point(vec![o.lng, o.lat]));
            let mut props = JsonObject::new();
            props.insert("origin_id".to_string(), serde_json::json!(o.origin_id));
            props.insert("num_students".to_string(), serde_json::json!(o.num_students));
            props.insert("pcycle_godutch".to_string(), serde_json::json!(o.pcycle_godutch));
            props.insert("bicycle_godutch".to_string(), serde_json::json!(o.bicycle_godutch));
            props.insert("assigned_route_rank".to_string(), serde_json::json!(o.assigned_route_rank));
            props.insert("dist_to_bike_bus_m".to_string(), serde_json::json!(o.dist_to_bike_bus_m));
            props.insert("bike_bus_length_m".to_string(), serde_json::json!(o.bike_bus_length_m));
            props.insert("total_route_length_m".to_string(), serde_json::json!(o.total_route_length_m));

            Feature {
                bbox: None,
                geometry: Some(geom),
                id: None,
                properties: Some(props),
                foreign_members: None,
            }
        })
        .collect();

    FeatureCollection {
        bbox: None,
        features,
        foreign_members: None,
    }
}
