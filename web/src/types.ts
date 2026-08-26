import type { FeatureCollection } from 'geojson';

export type RoutingProfile = 'quiet' | 'fast';

export interface OriginInput {
  id: string;
  lng: number;
  lat: number;
  num_students: number;
}

export interface BiciConfig {
  school_lng: number;
  school_lat: number;
  school_name?: string;
  origins: OriginInput[];
  routing_profile: RoutingProfile;
  min_trips_threshold: number;
  origin_buffer_m: number;
  max_routes: number;
  max_dist_to_bikebus_m: number;
  target_arrival_time: string;
  group_speed_kmh: number;
  dwell_time_mins: number;
  max_route_distance_m: number;
  circuity: number;
  max_straight_line_dist_m: number;
  max_shared_overlap_pct: number;
  min_dist_to_school_m: number;
  seed?: number;
}

export interface CandidateRoute {
  id: number;
  rank: number;
  origin_id: string;
  start_lng: number;
  start_lat: number;
  total_length_m: number;
  corridor_length_m: number;
  score: number;
  mean_godutch_demand: number;
  quietness_score: number;
  accommodated_students: number;
  accommodated_godutch: number;
}

export interface TimetableStop {
  stop_id: string;
  stop_name: string;
  stop_label: string;
  lng: number;
  lat: number;
  cumulative_dist_m: number;
  distance_to_next_m: number;
  arrival_time: string;
  departure_time: string;
  boarding_students: number;
  cumulative_students: number;
}

export interface RouteTimetable {
  route_rank: number;
  total_distance_m: number;
  total_duration_mins: number;
  average_speed_kmh: number;
  departure_time: string;
  arrival_time: string;
  stops: TimetableStop[];
}

export interface PlanningSummary {
  total_origins: number;
  total_students: number;
  total_godutch_potential: number;
  accommodated_students: number;
  accommodated_students_pct: number;
  accommodated_godutch: number;
  accommodated_godutch_pct: number;
  median_dist_to_bike_bus_m: number;
  mean_dist_to_bike_bus_m: number;
  total_network_km: number;
}

export interface BiciAnalysisOutput {
  routes_count: number;
  candidate_routes: CandidateRoute[];
  candidate_routes_geojson: FeatureCollection;
  route_network_geojson: FeatureCollection;
  matched_origins_geojson: FeatureCollection;
  timetables: RouteTimetable[];
  summary: PlanningSummary;
}

export interface SchoolInfo {
  name: string;
  dgeec_id?: number | string;
  lng: number;
  lat: number;
  total_students?: number;
}

export interface CaseStudyPreset {
  id: string;
  name: string;
  country: string;
  city: string;
  description: string;
  school: SchoolInfo;
  cents?: FeatureCollection;
  rnet?: FeatureCollection;
  candidate_routes?: FeatureCollection;
  actual_stops?: any[];
  actual_stops_geojson?: FeatureCollection;
  routes?: FeatureCollection;
}
