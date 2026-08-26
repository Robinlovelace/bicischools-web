use anyhow::Context;
use bicischools_core::{BiciConfig, BiciEngine, RoutingProfile};
use clap::Parser;
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "bicischools", about = "Bike Bus route planning and prioritisation CLI")]
struct Args {
    /// Path to OSM JSON file (exported from Overpass)
    #[arg(short, long)]
    osm_file: Option<PathBuf>,

    /// Target school longitude
    #[arg(long, default_value_t = -9.12191)]
    school_lng: f64,

    /// Target school latitude
    #[arg(long, default_value_t = 38.76714)]
    school_lat: f64,

    /// Routing profile: "quiet" or "fast"
    #[arg(long, default_value = "quiet")]
    profile: String,

    /// Minimum Go Dutch trip threshold for corridor selection
    #[arg(long, default_value_t = 3.0)]
    min_trips: f64,

    /// Maximum candidate routes to generate
    #[arg(long, default_value_t = 3)]
    max_routes: usize,

    /// Output JSON path
    #[arg(short, long)]
    output: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = Args::parse();

    println!("🚲 Bicischools Bike Bus Planning CLI");
    println!("School Coordinates: ({}, {})", args.school_lng, args.school_lat);

    let osm_data = if let Some(osm_path) = args.osm_file {
        println!("Reading OSM data from {:?}...", osm_path);
        fs::read_to_string(&osm_path).context("Failed to read OSM JSON file")?
    } else {
        anyhow::bail!("Please specify an OSM file with --osm-file <path>");
    };

    println!("Initializing routing graph...");
    let engine = BiciEngine::from_overpass_json(&osm_data)?;
    println!(
        "Graph built with {} nodes and {} edges.",
        engine.graph.nodes.len(),
        engine.graph.edges.len()
    );

    let routing_profile = if args.profile.to_lowercase() == "fast" {
        RoutingProfile::Fast
    } else {
        RoutingProfile::Quiet
    };

    let config = BiciConfig {
        school_lng: args.school_lng,
        school_lat: args.school_lat,
        routing_profile,
        min_trips_threshold: args.min_trips,
        max_routes: args.max_routes,
        ..Default::default()
    };

    println!("Running bike bus analysis...");
    let result = engine.run_analysis(&config)?;

    println!("\n=== Analysis Results ===");
    println!("Routes Evaluated: {}", result.routes_count);
    println!("Candidate Bike Buses Generated: {}", result.candidate_routes.len());
    println!("Total Students in Catchment: {:.1}", result.summary.total_students);
    println!(
        "Students Accommodated: {:.1} ({:.1}%)",
        result.summary.accommodated_students, result.summary.accommodated_students_pct
    );
    println!(
        "Go Dutch Cyclists Accommodated: {:.1} ({:.1}%)",
        result.summary.accommodated_godutch, result.summary.accommodated_godutch_pct
    );
    println!(
        "Median Walk/Ride to Bike Bus: {:.0}m",
        result.summary.median_dist_to_bike_bus_m
    );
    println!("Total Bike Bus Network: {:.2} km", result.summary.total_network_km);

    if let Some(out_path) = args.output {
        let serialized = serde_json::to_string_pretty(&result)?;
        fs::write(&out_path, serialized)?;
        println!("\nSaved results to {:?}", out_path);
    }

    Ok(())
}
