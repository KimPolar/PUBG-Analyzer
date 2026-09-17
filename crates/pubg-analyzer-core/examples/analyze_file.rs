use std::{env, fs};

use pubg_analyzer_core::telemetry::decode_events;
use pubg_analyzer_core::{AnalysisConfig, analyze_match};
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args()
        .nth(1)
        .ok_or("usage: analyze_file <telemetry.json[.gz]>")?;
    let payload = fs::read(&path)?;
    let events = decode_events(&payload)?;
    let analysis = analyze_match("local-file", &events, AnalysisConfig::default())?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "mapName": analysis.map_name,
            "eventCount": analysis.event_count,
            "durationSeconds": analysis.duration_seconds,
            "observedPlayers": analysis.observed_players,
            "observedTeams": analysis.observed_teams,
            "circleCount": analysis.circles.len(),
            "teamSnapshots": analysis.team_snapshot_count,
            "movementSegments": analysis.movement_segment_count,
            "phaseCells": analysis.cells.len(),
            "trainingRows": analysis.training_rows.len(),
        }))?
    );
    Ok(())
}
