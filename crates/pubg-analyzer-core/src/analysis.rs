use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::DateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{AnalyzerError, Result};

const CM_PER_METER: f64 = 100.0;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct AnalysisConfig {
    pub cell_size_m: f64,
    pub bucket_seconds: f64,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            cell_size_m: 100.0,
            bucket_seconds: 10.0,
        }
    }
}

impl AnalysisConfig {
    pub fn validate(self) -> Result<Self> {
        if !(10.0..=1_000.0).contains(&self.cell_size_m) {
            return Err(AnalyzerError::InvalidConfiguration(
                "cell size must be between 10m and 1000m".to_owned(),
            ));
        }
        if !(1.0..=60.0).contains(&self.bucket_seconds) {
            return Err(AnalyzerError::InvalidConfiguration(
                "bucket size must be between 1s and 60s".to_owned(),
            ));
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CircleState {
    pub phase: i32,
    pub center_x_m: f64,
    pub center_y_m: f64,
    pub radius_m: f64,
    pub first_seen_s: f64,
    pub last_seen_s: f64,
    pub sample_count: usize,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ObservedRate {
    pub rate: f64,
    pub observations: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PhaseCell {
    pub phase_cell_id: String,
    pub cell_id: String,
    pub phase: i32,
    pub cell_x: i32,
    pub cell_y: i32,
    pub center_x_m: f64,
    pub center_y_m: f64,
    pub occupancy_samples: usize,
    pub unique_teams: usize,
    pub visits: usize,
    pub mean_hold_seconds: Option<f64>,
    pub vehicle_sample_rate: f64,
    pub mean_team_spread_m: f64,
    pub survival_30s: Option<ObservedRate>,
    pub survival_60s: Option<ObservedRate>,
    pub survival_120s: Option<ObservedRate>,
    pub next_zone_retention: Option<ObservedRate>,
    pub mean_next_zone_entry_distance_m: Option<f64>,
    pub mean_enemy_teams_100m: f64,
    pub mean_enemy_teams_300m: f64,
    pub mean_enemy_teams_500m: f64,
    pub damage_dealt: f64,
    pub damage_received: f64,
    pub knocks_given: usize,
    pub knocks_received: usize,
    pub kills: usize,
    pub deaths: usize,
    pub estimated_ground_z_m: f64,
    pub relative_elevation_300m: Option<f64>,
    pub z_iqr_m: f64,
    pub historical_value_score: f64,
    pub score_confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TrainingRow {
    pub match_id: String,
    pub map_name: String,
    pub phase: i32,
    pub relative_x: Option<f64>,
    pub relative_y: Option<f64>,
    pub normalized_center_distance: Option<f64>,
    pub distance_to_edge_m: Option<f64>,
    pub next_zone_entry_distance_m: Option<f64>,
    pub member_count: usize,
    pub spread_m: f64,
    pub vehicle_member_rate: f64,
    pub enemy_teams_100m: usize,
    pub enemy_teams_300m: usize,
    pub enemy_teams_500m: usize,
    pub relative_elevation_300m: Option<f64>,
    pub next_zone_contains: Option<bool>,
    pub survives_30s: Option<bool>,
    pub survives_60s: Option<bool>,
    pub survives_120s: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MatchAnalysis {
    pub schema_version: String,
    pub match_id: String,
    pub map_name: String,
    pub event_count: usize,
    pub duration_seconds: f64,
    pub observed_players: usize,
    pub observed_teams: usize,
    pub team_snapshot_count: usize,
    pub movement_segment_count: usize,
    pub circles: Vec<CircleState>,
    pub cells: Vec<PhaseCell>,
    pub training_rows: Vec<TrainingRow>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DatasetAnalysis {
    pub match_count: usize,
    pub training_row_count: usize,
    pub maps: Vec<MapDatasetAnalysis>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MapDatasetAnalysis {
    pub map_name: String,
    pub match_count: usize,
    pub cells: Vec<DatasetCell>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DatasetCell {
    pub phase_cell_id: String,
    pub cell_id: String,
    pub phase: i32,
    pub cell_x: i32,
    pub cell_y: i32,
    pub center_x_m: f64,
    pub center_y_m: f64,
    pub match_count: usize,
    pub occupancy_samples: usize,
    pub visits: usize,
    pub survival_120s: Option<f64>,
    pub next_zone_retention: Option<f64>,
    pub mean_hold_seconds: Option<f64>,
    pub mean_enemy_teams_300m: f64,
    pub damage_balance: f64,
    pub historical_value_score: f64,
    pub score_confidence: f64,
}

#[derive(Debug, Clone, Copy, Default)]
struct Point {
    x: f64,
    y: f64,
    z: f64,
}

impl Point {
    fn from_value(value: Option<&Value>) -> Option<Self> {
        let value = value?.as_object()?;
        Some(Self {
            x: value.get("x")?.as_f64()?,
            y: value.get("y")?.as_f64()?,
            z: value.get("z").and_then(Value::as_f64).unwrap_or(0.0),
        })
    }

    fn distance_2d(self, other: Self) -> f64 {
        (self.x - other.x).hypot(self.y - other.y)
    }
}

#[derive(Debug, Clone)]
struct CircleInternal {
    phase: i32,
    center: Point,
    radius: f64,
    first_seen: f64,
    last_seen: f64,
    sample_count: usize,
    source: &'static str,
}

impl CircleInternal {
    fn contains(&self, point: Point) -> bool {
        self.center.distance_2d(point) <= self.radius
    }

    fn distance_to_edge_m(&self, point: Point) -> f64 {
        (self.radius - self.center.distance_2d(point)) / CM_PER_METER
    }
}

#[derive(Debug, Clone)]
struct Snapshot {
    bucket: i64,
    elapsed: f64,
    phase: i32,
    team_id: i64,
    center: Point,
    member_count: usize,
    spread_m: f64,
    vehicle_members: usize,
    cell_x: i32,
    cell_y: i32,
    distance_to_edge_m: Option<f64>,
    normalized_center_distance: Option<f64>,
    relative_x: Option<f64>,
    relative_y: Option<f64>,
    next_zone_contains: Option<bool>,
    next_zone_distance_m: Option<f64>,
    enemy_100m: usize,
    enemy_300m: usize,
    enemy_500m: usize,
    survives_30s: Option<bool>,
    survives_60s: Option<bool>,
    survives_120s: Option<bool>,
}

#[derive(Debug, Clone, Copy)]
struct MemberObservation {
    elapsed: f64,
    position: Point,
    in_vehicle: bool,
}

#[derive(Debug, Clone, Copy, Default)]
struct CombatTotals {
    damage_dealt: f64,
    damage_received: f64,
    knocks_given: usize,
    knocks_received: usize,
    kills: usize,
    deaths: usize,
}

#[derive(Default)]
struct CellAccumulator {
    snapshots: Vec<usize>,
    unique_teams: HashSet<i64>,
    hold_durations: Vec<f64>,
    combat: CombatTotals,
}

struct EventClock {
    origin_epoch: Option<f64>,
}

impl EventClock {
    fn from_events(events: &[Value]) -> Self {
        let preferred: Vec<f64> = events
            .iter()
            .filter(|event| event_type(event) == Some("LogGameStatePeriodic"))
            .filter_map(|event| Some(timestamp_epoch(event.get("_D")?)? - explicit_elapsed(event)?))
            .collect();
        let fallback: Vec<f64> = events
            .iter()
            .filter_map(|event| Some(timestamp_epoch(event.get("_D")?)? - explicit_elapsed(event)?))
            .collect();
        let anchors = if preferred.is_empty() {
            fallback
        } else {
            preferred
        };
        Self {
            origin_epoch: median(&anchors),
        }
    }

    fn elapsed(&self, event: &Value) -> Option<f64> {
        if let (Some(origin), Some(timestamp)) =
            (self.origin_epoch, event.get("_D").and_then(timestamp_epoch))
        {
            return Some(timestamp - origin);
        }
        explicit_elapsed(event)
    }
}

pub fn analyze_match(
    match_id: &str,
    events: &[Value],
    config: AnalysisConfig,
) -> Result<MatchAnalysis> {
    let config = config.validate()?;
    if events.is_empty() {
        return Err(AnalyzerError::InvalidTelemetry(
            "telemetry contains no events".to_owned(),
        ));
    }
    let clock = EventClock::from_events(events);
    let phases = build_phase_timeline(events, &clock);
    let circles = build_circles(events, &clock, &phases);
    let duration = match_end(events, &clock);
    let map_name = events
        .iter()
        .find(|event| event_type(event) == Some("LogMatchStart"))
        .and_then(|event| event.get("mapName"))
        .and_then(Value::as_str)
        .unwrap_or("Unknown")
        .to_owned();
    let mut snapshots = build_snapshots(events, &clock, &phases, &circles, config);
    add_enemy_density(&mut snapshots);
    add_future_labels(&mut snapshots, duration);
    let movement_segment_count = movement_segment_count(&snapshots, config.bucket_seconds);
    let (cells, elevation) = build_cells(events, &clock, &phases, &snapshots, config);

    let training_rows = snapshots
        .iter()
        .map(|snapshot| TrainingRow {
            match_id: match_id.to_owned(),
            map_name: map_name.clone(),
            phase: snapshot.phase,
            relative_x: rounded_option(snapshot.relative_x, 5),
            relative_y: rounded_option(snapshot.relative_y, 5),
            normalized_center_distance: rounded_option(snapshot.normalized_center_distance, 5),
            distance_to_edge_m: rounded_option(snapshot.distance_to_edge_m, 3),
            next_zone_entry_distance_m: rounded_option(snapshot.next_zone_distance_m, 3),
            member_count: snapshot.member_count,
            spread_m: round(snapshot.spread_m, 3),
            vehicle_member_rate: round(
                snapshot.vehicle_members as f64 / snapshot.member_count.max(1) as f64,
                4,
            ),
            enemy_teams_100m: snapshot.enemy_100m,
            enemy_teams_300m: snapshot.enemy_300m,
            enemy_teams_500m: snapshot.enemy_500m,
            relative_elevation_300m: elevation
                .get(&(snapshot.cell_x, snapshot.cell_y))
                .and_then(|item| item.relative),
            next_zone_contains: snapshot.next_zone_contains,
            survives_30s: snapshot.survives_30s,
            survives_60s: snapshot.survives_60s,
            survives_120s: snapshot.survives_120s,
        })
        .collect();

    let players: HashSet<String> = events
        .iter()
        .filter_map(|event| event.get("character"))
        .filter_map(|character| character.get("accountId").or_else(|| character.get("name")))
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect();
    let teams: HashSet<i64> = snapshots.iter().map(|snapshot| snapshot.team_id).collect();

    Ok(MatchAnalysis {
        schema_version: "0.2.0".to_owned(),
        match_id: match_id.to_owned(),
        map_name,
        event_count: events.len(),
        duration_seconds: round(duration, 3),
        observed_players: players.len(),
        observed_teams: teams.len(),
        team_snapshot_count: snapshots.len(),
        movement_segment_count,
        circles: circles.iter().map(circle_to_public).collect(),
        cells,
        training_rows,
    })
}

#[must_use]
pub fn analyze_dataset(matches: &[MatchAnalysis]) -> DatasetAnalysis {
    let mut grouped: BTreeMap<String, Vec<&MatchAnalysis>> = BTreeMap::new();
    for analysis in matches {
        grouped
            .entry(analysis.map_name.clone())
            .or_default()
            .push(analysis);
    }

    let maps = grouped
        .into_iter()
        .map(|(map_name, analyses)| {
            let mut cells: BTreeMap<(i32, i32, i32), Vec<&PhaseCell>> = BTreeMap::new();
            for analysis in &analyses {
                for cell in &analysis.cells {
                    cells
                        .entry((cell.phase, cell.cell_x, cell.cell_y))
                        .or_default()
                        .push(cell);
                }
            }
            let mut cells: Vec<DatasetCell> = cells
                .into_iter()
                .map(|((phase, cell_x, cell_y), values)| {
                    aggregate_dataset_cell(phase, cell_x, cell_y, &values)
                })
                .collect();
            cells.sort_by(|left, right| {
                right
                    .historical_value_score
                    .total_cmp(&left.historical_value_score)
                    .then_with(|| right.occupancy_samples.cmp(&left.occupancy_samples))
            });
            MapDatasetAnalysis {
                map_name,
                match_count: analyses.len(),
                cells,
            }
        })
        .collect();

    DatasetAnalysis {
        match_count: matches.len(),
        training_row_count: matches
            .iter()
            .map(|analysis| analysis.training_rows.len())
            .sum(),
        maps,
    }
}

fn aggregate_dataset_cell(
    phase: i32,
    cell_x: i32,
    cell_y: i32,
    values: &[&PhaseCell],
) -> DatasetCell {
    let weight_sum: usize = values.iter().map(|cell| cell.occupancy_samples).sum();
    let visits: usize = values.iter().map(|cell| cell.visits).sum();
    let weighted = |extract: fn(&PhaseCell) -> f64| {
        if weight_sum == 0 {
            0.0
        } else {
            values
                .iter()
                .map(|cell| extract(cell) * cell.occupancy_samples as f64)
                .sum::<f64>()
                / weight_sum as f64
        }
    };
    let survival = aggregate_rate(values.iter().filter_map(|cell| cell.survival_120s.as_ref()));
    let retention = aggregate_rate(
        values
            .iter()
            .filter_map(|cell| cell.next_zone_retention.as_ref()),
    );
    let hold = weighted_optional(
        values
            .iter()
            .filter_map(|cell| cell.mean_hold_seconds.map(|value| (value, cell.visits))),
    );
    let score = weighted(|cell| cell.historical_value_score);
    let match_confidence = 1.0 - (-(values.len() as f64) / 12.0).exp();
    let confidence = (1.0 - (-(visits as f64) / 30.0).exp()) * match_confidence;
    DatasetCell {
        phase_cell_id: format!("p{phase}:{cell_x}:{cell_y}"),
        cell_id: format!("{cell_x}:{cell_y}"),
        phase,
        cell_x,
        cell_y,
        center_x_m: values.first().map_or(0.0, |cell| cell.center_x_m),
        center_y_m: values.first().map_or(0.0, |cell| cell.center_y_m),
        match_count: values.len(),
        occupancy_samples: weight_sum,
        visits,
        survival_120s: survival.map(|rate| round(rate.rate, 4)),
        next_zone_retention: retention.map(|rate| round(rate.rate, 4)),
        mean_hold_seconds: rounded_option(hold, 3),
        mean_enemy_teams_300m: round(weighted(|cell| cell.mean_enemy_teams_300m), 4),
        damage_balance: round(
            values
                .iter()
                .map(|cell| cell.damage_dealt - cell.damage_received)
                .sum(),
            3,
        ),
        historical_value_score: round(50.0 + confidence * (score - 50.0), 2),
        score_confidence: round(confidence, 4),
    }
}

fn build_phase_timeline(events: &[Value], clock: &EventClock) -> Vec<(f64, i32)> {
    let mut markers: Vec<(f64, i32)> = events
        .iter()
        .filter(|event| event_type(event) == Some("LogPhaseChange"))
        .filter_map(|event| {
            Some((
                clock.elapsed(event)?.max(0.0),
                event.get("phase")?.as_i64()? as i32,
            ))
        })
        .collect();
    markers.sort_by(|left, right| left.0.total_cmp(&right.0));
    markers.dedup_by(|current, previous| current.1 == previous.1);
    markers
}

fn phase_at(markers: &[(f64, i32)], elapsed: f64) -> i32 {
    markers
        .iter()
        .take_while(|(time, _)| *time <= elapsed)
        .last()
        .map_or(0, |(_, phase)| *phase)
}

fn build_circles(
    events: &[Value],
    clock: &EventClock,
    phases: &[(f64, i32)],
) -> Vec<CircleInternal> {
    let mut initial = Vec::new();
    let mut warnings: BTreeMap<i32, Vec<(f64, Point, f64)>> = BTreeMap::new();
    for event in events
        .iter()
        .filter(|event| event_type(event) == Some("LogGameStatePeriodic"))
    {
        let Some(elapsed) = clock.elapsed(event) else {
            continue;
        };
        let safety = Point::from_value(event.pointer("/gameState/safetyZonePosition"));
        let safety_radius = event
            .pointer("/gameState/safetyZoneRadius")
            .and_then(Value::as_f64);
        if let (Some(center), Some(radius)) = (safety, safety_radius)
            && radius > 0.0
        {
            initial.push((elapsed, center, radius));
        }
        let phase = phase_at(phases, elapsed);
        let warning = Point::from_value(event.pointer("/gameState/poisonGasWarningPosition"));
        let warning_radius = event
            .pointer("/gameState/poisonGasWarningRadius")
            .and_then(Value::as_f64);
        if phase > 0
            && let (Some(center), Some(radius)) = (warning, warning_radius)
            && radius > 0.0
        {
            warnings
                .entry(phase)
                .or_default()
                .push((elapsed, center, radius));
        }
    }

    let mut result = Vec::new();
    if !initial.is_empty() {
        let first_phase_time = phases.first().map_or(f64::INFINITY, |item| item.0);
        let before: Vec<_> = initial
            .iter()
            .copied()
            .filter(|sample| sample.0 <= first_phase_time)
            .collect();
        let selected = if before.is_empty() {
            vec![initial[0]]
        } else {
            before
        };
        result.push(representative_circle(0, &selected, "initial_safety_zone"));
    }
    result.extend(
        warnings.into_iter().map(|(phase, samples)| {
            representative_circle(phase, &samples, "poison_gas_warning_zone")
        }),
    );
    result
}

fn representative_circle(
    phase: i32,
    samples: &[(f64, Point, f64)],
    source: &'static str,
) -> CircleInternal {
    CircleInternal {
        phase,
        center: Point {
            x: median(&samples.iter().map(|sample| sample.1.x).collect::<Vec<_>>())
                .unwrap_or_default(),
            y: median(&samples.iter().map(|sample| sample.1.y).collect::<Vec<_>>())
                .unwrap_or_default(),
            z: median(&samples.iter().map(|sample| sample.1.z).collect::<Vec<_>>())
                .unwrap_or_default(),
        },
        radius: median(&samples.iter().map(|sample| sample.2).collect::<Vec<_>>())
            .unwrap_or_default(),
        first_seen: samples
            .iter()
            .map(|sample| sample.0)
            .fold(f64::INFINITY, f64::min),
        last_seen: samples
            .iter()
            .map(|sample| sample.0)
            .fold(f64::NEG_INFINITY, f64::max),
        sample_count: samples.len(),
        source,
    }
}

fn build_snapshots(
    events: &[Value],
    clock: &EventClock,
    phases: &[(f64, i32)],
    circles: &[CircleInternal],
    config: AnalysisConfig,
) -> Vec<Snapshot> {
    let mut grouped: BTreeMap<(i64, i64), HashMap<String, MemberObservation>> = BTreeMap::new();
    for (index, event) in events.iter().enumerate() {
        if event_type(event) != Some("LogPlayerPosition") {
            continue;
        }
        let (Some(elapsed), Some(character)) = (clock.elapsed(event), event.get("character"))
        else {
            continue;
        };
        if elapsed < 0.0 {
            continue;
        }
        let (Some(team_id), Some(position)) = (
            character.get("teamId").and_then(Value::as_i64),
            Point::from_value(character.get("location")),
        ) else {
            continue;
        };
        let player_key = character
            .get("accountId")
            .or_else(|| character.get("name"))
            .and_then(Value::as_str)
            .map_or_else(|| format!("event-{index}"), ToOwned::to_owned);
        let in_vehicle = character
            .get("isInVehicle")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            || event
                .get("vehicle")
                .is_some_and(|vehicle| !vehicle.is_null());
        let bucket = (elapsed / config.bucket_seconds).floor() as i64;
        let observation = MemberObservation {
            elapsed,
            position,
            in_vehicle,
        };
        let existing = grouped
            .entry((bucket, team_id))
            .or_default()
            .entry(player_key)
            .or_insert(observation);
        if existing.elapsed <= elapsed {
            *existing = observation;
        }
    }

    let circle_by_phase: BTreeMap<i32, &CircleInternal> = circles
        .iter()
        .map(|circle| (circle.phase, circle))
        .collect();
    let circle_phases: Vec<i32> = circle_by_phase.keys().copied().collect();
    let cell_size_cm = config.cell_size_m * CM_PER_METER;
    grouped
        .into_iter()
        .filter_map(|((bucket, team_id), member_map)| {
            let members: Vec<MemberObservation> = member_map.into_values().collect();
            if members.is_empty() {
                return None;
            }
            let elapsed = median(
                &members
                    .iter()
                    .map(|member| member.elapsed)
                    .collect::<Vec<_>>(),
            )?;
            let center = Point {
                x: median(
                    &members
                        .iter()
                        .map(|member| member.position.x)
                        .collect::<Vec<_>>(),
                )?,
                y: median(
                    &members
                        .iter()
                        .map(|member| member.position.y)
                        .collect::<Vec<_>>(),
                )?,
                z: median(
                    &members
                        .iter()
                        .map(|member| member.position.z)
                        .collect::<Vec<_>>(),
                )?,
            };
            let spread_m = members
                .iter()
                .map(|member| center.distance_2d(member.position) / CM_PER_METER)
                .fold(0.0, f64::max);
            let phase = phase_at(phases, elapsed);
            let mut snapshot = Snapshot {
                bucket,
                elapsed,
                phase,
                team_id,
                center,
                member_count: members.len(),
                spread_m,
                vehicle_members: members.iter().filter(|member| member.in_vehicle).count(),
                cell_x: (center.x / cell_size_cm).floor() as i32,
                cell_y: (center.y / cell_size_cm).floor() as i32,
                distance_to_edge_m: None,
                normalized_center_distance: None,
                relative_x: None,
                relative_y: None,
                next_zone_contains: None,
                next_zone_distance_m: None,
                enemy_100m: 0,
                enemy_300m: 0,
                enemy_500m: 0,
                survives_30s: None,
                survives_60s: None,
                survives_120s: None,
            };
            add_zone_features(&mut snapshot, &circle_by_phase, &circle_phases);
            Some(snapshot)
        })
        .collect()
}

fn add_zone_features(
    snapshot: &mut Snapshot,
    circles: &BTreeMap<i32, &CircleInternal>,
    phases: &[i32],
) {
    let current = circles.get(&snapshot.phase).copied().or_else(|| {
        phases
            .iter()
            .rfind(|phase| **phase <= snapshot.phase)
            .and_then(|phase| circles.get(phase).copied())
    });
    if let Some(circle) = current {
        let distance = circle.center.distance_2d(snapshot.center);
        snapshot.distance_to_edge_m = Some((circle.radius - distance) / CM_PER_METER);
        snapshot.normalized_center_distance =
            (circle.radius > 0.0).then_some(distance / circle.radius);
        snapshot.relative_x =
            (circle.radius > 0.0).then_some((snapshot.center.x - circle.center.x) / circle.radius);
        snapshot.relative_y =
            (circle.radius > 0.0).then_some((snapshot.center.y - circle.center.y) / circle.radius);
    }
    if let Some(next) = phases
        .iter()
        .find(|phase| **phase > snapshot.phase)
        .and_then(|phase| circles.get(phase))
    {
        snapshot.next_zone_contains = Some(next.contains(snapshot.center));
        snapshot.next_zone_distance_m = Some((-next.distance_to_edge_m(snapshot.center)).max(0.0));
    }
}

fn add_enemy_density(snapshots: &mut [Snapshot]) {
    let mut buckets: BTreeMap<i64, Vec<usize>> = BTreeMap::new();
    for (index, snapshot) in snapshots.iter().enumerate() {
        buckets.entry(snapshot.bucket).or_default().push(index);
    }
    for indices in buckets.into_values() {
        for left_position in 0..indices.len() {
            for right_position in (left_position + 1)..indices.len() {
                let left_index = indices[left_position];
                let right_index = indices[right_position];
                if snapshots[left_index].team_id == snapshots[right_index].team_id {
                    continue;
                }
                let distance = snapshots[left_index]
                    .center
                    .distance_2d(snapshots[right_index].center)
                    / CM_PER_METER;
                if distance <= 500.0 {
                    snapshots[left_index].enemy_500m += 1;
                    snapshots[right_index].enemy_500m += 1;
                }
                if distance <= 300.0 {
                    snapshots[left_index].enemy_300m += 1;
                    snapshots[right_index].enemy_300m += 1;
                }
                if distance <= 100.0 {
                    snapshots[left_index].enemy_100m += 1;
                    snapshots[right_index].enemy_100m += 1;
                }
            }
        }
    }
}

fn add_future_labels(snapshots: &mut [Snapshot], match_end: f64) {
    let mut teams: BTreeMap<i64, Vec<usize>> = BTreeMap::new();
    for (index, snapshot) in snapshots.iter().enumerate() {
        teams.entry(snapshot.team_id).or_default().push(index);
    }
    for indices in teams.values_mut() {
        indices.sort_by(|left, right| {
            snapshots[*left]
                .elapsed
                .total_cmp(&snapshots[*right].elapsed)
        });
        let times: Vec<f64> = indices
            .iter()
            .map(|index| snapshots[*index].elapsed)
            .collect();
        for (position, snapshot_index) in indices.iter().copied().enumerate() {
            let elapsed = snapshots[snapshot_index].elapsed;
            snapshots[snapshot_index].survives_30s =
                survival_label(&times, position, elapsed, 30.0, match_end);
            snapshots[snapshot_index].survives_60s =
                survival_label(&times, position, elapsed, 60.0, match_end);
            snapshots[snapshot_index].survives_120s =
                survival_label(&times, position, elapsed, 120.0, match_end);
        }
    }
}

fn survival_label(
    times: &[f64],
    current_position: usize,
    elapsed: f64,
    horizon: f64,
    match_end: f64,
) -> Option<bool> {
    let target = elapsed + horizon;
    if target > match_end {
        return None;
    }
    Some(
        times[(current_position + 1)..]
            .iter()
            .any(|time| *time >= target),
    )
}

fn movement_segment_count(snapshots: &[Snapshot], bucket_seconds: f64) -> usize {
    let mut teams: BTreeMap<i64, Vec<&Snapshot>> = BTreeMap::new();
    for snapshot in snapshots {
        teams.entry(snapshot.team_id).or_default().push(snapshot);
    }
    teams
        .values_mut()
        .map(|values| {
            values.sort_by(|left, right| left.elapsed.total_cmp(&right.elapsed));
            values
                .windows(2)
                .filter(|pair| {
                    let duration = pair[1].elapsed - pair[0].elapsed;
                    duration > 0.0 && duration <= bucket_seconds * 3.5
                })
                .count()
        })
        .sum()
}

#[derive(Debug, Clone, Copy)]
struct ElevationCell {
    ground: f64,
    relative: Option<f64>,
    iqr: f64,
}

fn build_cells(
    events: &[Value],
    clock: &EventClock,
    phases: &[(f64, i32)],
    snapshots: &[Snapshot],
    config: AnalysisConfig,
) -> (Vec<PhaseCell>, BTreeMap<(i32, i32), ElevationCell>) {
    let mut accumulators: BTreeMap<(i32, i32, i32), CellAccumulator> = BTreeMap::new();
    for (index, snapshot) in snapshots.iter().enumerate() {
        let accumulator = accumulators
            .entry((snapshot.phase, snapshot.cell_x, snapshot.cell_y))
            .or_default();
        accumulator.snapshots.push(index);
        accumulator.unique_teams.insert(snapshot.team_id);
    }
    add_hold_durations(&mut accumulators, snapshots, config.bucket_seconds);
    for (key, combat) in collect_combat(events, clock, phases, config.cell_size_m) {
        accumulators.entry(key).or_default().combat = combat;
    }
    let elevation = build_elevation_map(snapshots, config.cell_size_m);
    let mut cells: Vec<PhaseCell> = accumulators
        .into_iter()
        .filter_map(|((phase, cell_x, cell_y), accumulator)| {
            (!accumulator.snapshots.is_empty()).then(|| {
                cell_to_public(
                    phase,
                    cell_x,
                    cell_y,
                    accumulator,
                    snapshots,
                    &elevation,
                    config.cell_size_m,
                )
            })
        })
        .collect();
    cells.sort_by(|left, right| {
        right
            .historical_value_score
            .total_cmp(&left.historical_value_score)
            .then_with(|| right.occupancy_samples.cmp(&left.occupancy_samples))
    });
    (cells, elevation)
}

fn add_hold_durations(
    accumulators: &mut BTreeMap<(i32, i32, i32), CellAccumulator>,
    snapshots: &[Snapshot],
    bucket_seconds: f64,
) {
    let mut teams: BTreeMap<i64, Vec<usize>> = BTreeMap::new();
    for (index, snapshot) in snapshots.iter().enumerate() {
        teams.entry(snapshot.team_id).or_default().push(index);
    }
    for indices in teams.values_mut() {
        indices.sort_by(|left, right| {
            snapshots[*left]
                .elapsed
                .total_cmp(&snapshots[*right].elapsed)
        });
        let mut run = Vec::new();
        for index in indices.iter().copied() {
            if let Some(previous_index) = run.last().copied() {
                let previous: &Snapshot = &snapshots[previous_index];
                let current = &snapshots[index];
                let same_cell = previous.phase == current.phase
                    && previous.cell_x == current.cell_x
                    && previous.cell_y == current.cell_y;
                let contiguous = current.elapsed - previous.elapsed <= bucket_seconds * 2.5;
                if !same_cell || !contiguous {
                    finish_hold_run(accumulators, snapshots, &run, bucket_seconds);
                    run.clear();
                }
            }
            run.push(index);
        }
        finish_hold_run(accumulators, snapshots, &run, bucket_seconds);
    }
}

fn finish_hold_run(
    accumulators: &mut BTreeMap<(i32, i32, i32), CellAccumulator>,
    snapshots: &[Snapshot],
    run: &[usize],
    bucket_seconds: f64,
) {
    let (Some(first_index), Some(last_index)) = (run.first(), run.last()) else {
        return;
    };
    let first = &snapshots[*first_index];
    let last = &snapshots[*last_index];
    let duration = (last.elapsed - first.elapsed + bucket_seconds).max(bucket_seconds);
    if let Some(accumulator) = accumulators.get_mut(&(first.phase, first.cell_x, first.cell_y)) {
        accumulator.hold_durations.push(duration);
    }
}

fn collect_combat(
    events: &[Value],
    clock: &EventClock,
    phases: &[(f64, i32)],
    cell_size_m: f64,
) -> BTreeMap<(i32, i32, i32), CombatTotals> {
    let mut totals: BTreeMap<(i32, i32, i32), CombatTotals> = BTreeMap::new();
    for event in events {
        let Some(kind) = event_type(event) else {
            continue;
        };
        if !matches!(
            kind,
            "LogPlayerTakeDamage" | "LogPlayerMakeGroggy" | "LogPlayerKillV2"
        ) {
            continue;
        }
        let Some(elapsed) = clock.elapsed(event) else {
            continue;
        };
        if elapsed < 0.0 {
            continue;
        }
        let phase = phase_at(phases, elapsed);
        match kind {
            "LogPlayerTakeDamage" => {
                let damage = event.get("damage").and_then(Value::as_f64).unwrap_or(0.0);
                if let Some(key) = character_cell(event.get("attacker"), phase, cell_size_m) {
                    totals.entry(key).or_default().damage_dealt += damage;
                }
                if let Some(key) = character_cell(event.get("victim"), phase, cell_size_m) {
                    totals.entry(key).or_default().damage_received += damage;
                }
            }
            "LogPlayerMakeGroggy" => {
                if let Some(key) = character_cell(event.get("attacker"), phase, cell_size_m) {
                    totals.entry(key).or_default().knocks_given += 1;
                }
                if let Some(key) = character_cell(event.get("victim"), phase, cell_size_m) {
                    totals.entry(key).or_default().knocks_received += 1;
                }
            }
            "LogPlayerKillV2" => {
                let killer = event
                    .get("killer")
                    .filter(|value| !value.is_null())
                    .or_else(|| event.get("finisher"));
                if let Some(key) = character_cell(killer, phase, cell_size_m) {
                    totals.entry(key).or_default().kills += 1;
                }
                if let Some(key) = character_cell(event.get("victim"), phase, cell_size_m) {
                    totals.entry(key).or_default().deaths += 1;
                }
            }
            _ => {}
        }
    }
    totals
}

fn character_cell(
    character: Option<&Value>,
    phase: i32,
    cell_size_m: f64,
) -> Option<(i32, i32, i32)> {
    let location = Point::from_value(character?.get("location"))?;
    let cell_size = cell_size_m * CM_PER_METER;
    Some((
        phase,
        (location.x / cell_size).floor() as i32,
        (location.y / cell_size).floor() as i32,
    ))
}

fn build_elevation_map(
    snapshots: &[Snapshot],
    cell_size_m: f64,
) -> BTreeMap<(i32, i32), ElevationCell> {
    let mut heights: BTreeMap<(i32, i32), Vec<f64>> = BTreeMap::new();
    for snapshot in snapshots {
        heights
            .entry((snapshot.cell_x, snapshot.cell_y))
            .or_default()
            .push(snapshot.center.z / CM_PER_METER);
    }
    let ground: BTreeMap<(i32, i32), f64> = heights
        .iter()
        .map(|(cell, values)| (*cell, percentile(values, 0.2)))
        .collect();
    let radius_cells = (300.0 / cell_size_m).round().max(1.0);
    ground
        .iter()
        .map(|(&(cell_x, cell_y), &ground_z)| {
            let neighbors: Vec<f64> = ground
                .iter()
                .filter(|((other_x, other_y), _)| {
                    (*other_x, *other_y) != (cell_x, cell_y)
                        && f64::from(*other_x - cell_x).hypot(f64::from(*other_y - cell_y))
                            <= radius_cells
                })
                .map(|(_, value)| *value)
                .collect();
            let relative = median(&neighbors).map(|neighbor| ground_z - neighbor);
            let values = &heights[&(cell_x, cell_y)];
            (
                (cell_x, cell_y),
                ElevationCell {
                    ground: ground_z,
                    relative,
                    iqr: percentile(values, 0.75) - percentile(values, 0.25),
                },
            )
        })
        .collect()
}

fn cell_to_public(
    phase: i32,
    cell_x: i32,
    cell_y: i32,
    accumulator: CellAccumulator,
    snapshots: &[Snapshot],
    elevation: &BTreeMap<(i32, i32), ElevationCell>,
    cell_size_m: f64,
) -> PhaseCell {
    let values: Vec<&Snapshot> = accumulator
        .snapshots
        .iter()
        .map(|index| &snapshots[*index])
        .collect();
    let visits = accumulator.hold_durations.len();
    let hold_mean = mean(&accumulator.hold_durations);
    let survival_30 = boolean_rate(values.iter().map(|snapshot| snapshot.survives_30s));
    let survival_60 = boolean_rate(values.iter().map(|snapshot| snapshot.survives_60s));
    let survival_120 = boolean_rate(values.iter().map(|snapshot| snapshot.survives_120s));
    let retention = boolean_rate(values.iter().map(|snapshot| snapshot.next_zone_contains));
    let next_distance = mean_optional(values.iter().map(|snapshot| snapshot.next_zone_distance_m));
    let enemy_300 = values
        .iter()
        .map(|snapshot| snapshot.enemy_300m as f64)
        .sum::<f64>()
        / values.len() as f64;
    let elevation = elevation
        .get(&(cell_x, cell_y))
        .copied()
        .unwrap_or(ElevationCell {
            ground: 0.0,
            relative: None,
            iqr: 0.0,
        });
    let (score, confidence) = historical_score(
        values.len(),
        visits,
        hold_mean,
        survival_120.as_ref(),
        retention.as_ref(),
        next_distance,
        enemy_300,
        accumulator.combat,
    );
    PhaseCell {
        phase_cell_id: format!("p{phase}:{cell_x}:{cell_y}"),
        cell_id: format!("{cell_x}:{cell_y}"),
        phase,
        cell_x,
        cell_y,
        center_x_m: round((f64::from(cell_x) + 0.5) * cell_size_m, 3),
        center_y_m: round((f64::from(cell_y) + 0.5) * cell_size_m, 3),
        occupancy_samples: values.len(),
        unique_teams: accumulator.unique_teams.len(),
        visits,
        mean_hold_seconds: rounded_option(hold_mean, 3),
        vehicle_sample_rate: round(
            values
                .iter()
                .filter(|snapshot| snapshot.vehicle_members > 0)
                .count() as f64
                / values.len() as f64,
            4,
        ),
        mean_team_spread_m: round(
            values.iter().map(|snapshot| snapshot.spread_m).sum::<f64>() / values.len() as f64,
            3,
        ),
        survival_30s: survival_30,
        survival_60s: survival_60,
        survival_120s: survival_120,
        next_zone_retention: retention,
        mean_next_zone_entry_distance_m: rounded_option(next_distance, 3),
        mean_enemy_teams_100m: round(
            values
                .iter()
                .map(|snapshot| snapshot.enemy_100m as f64)
                .sum::<f64>()
                / values.len() as f64,
            4,
        ),
        mean_enemy_teams_300m: round(enemy_300, 4),
        mean_enemy_teams_500m: round(
            values
                .iter()
                .map(|snapshot| snapshot.enemy_500m as f64)
                .sum::<f64>()
                / values.len() as f64,
            4,
        ),
        damage_dealt: round(accumulator.combat.damage_dealt, 3),
        damage_received: round(accumulator.combat.damage_received, 3),
        knocks_given: accumulator.combat.knocks_given,
        knocks_received: accumulator.combat.knocks_received,
        kills: accumulator.combat.kills,
        deaths: accumulator.combat.deaths,
        estimated_ground_z_m: round(elevation.ground, 3),
        relative_elevation_300m: rounded_option(elevation.relative, 3),
        z_iqr_m: round(elevation.iqr, 3),
        historical_value_score: round(score, 2),
        score_confidence: round(confidence, 4),
    }
}

#[allow(clippy::too_many_arguments)]
fn historical_score(
    samples: usize,
    visits: usize,
    hold_mean: Option<f64>,
    survival_120: Option<&ObservedRate>,
    retention: Option<&ObservedRate>,
    next_distance: Option<f64>,
    enemy_300: f64,
    combat: CombatTotals,
) -> (f64, f64) {
    let survival = shrunk_rate(survival_120);
    let retention = shrunk_rate(retention);
    let hold = (hold_mean.unwrap_or_default() / 120.0).clamp(0.0, 1.0);
    let rotation = next_distance.map_or(0.5, |distance| (-distance / 500.0).exp());
    let net_damage = (combat.damage_dealt - combat.damage_received) / visits.max(1) as f64;
    let combat_balance = 0.5 + 0.5 * (net_damage / 100.0).tanh();
    let low_contest = 1.0 - (enemy_300 / 3.0).clamp(0.0, 1.0);
    let raw = 0.25 * survival
        + 0.20 * retention
        + 0.15 * hold
        + 0.15 * rotation
        + 0.15 * combat_balance
        + 0.10 * low_contest;
    let confidence = 1.0 - (-(visits.max(1).min(samples) as f64) / 8.0).exp();
    (50.0 + confidence * (raw * 100.0 - 50.0), confidence)
}

fn boolean_rate(values: impl Iterator<Item = Option<bool>>) -> Option<ObservedRate> {
    let observed: Vec<bool> = values.flatten().collect();
    (!observed.is_empty()).then(|| ObservedRate {
        rate: round(
            observed.iter().filter(|value| **value).count() as f64 / observed.len() as f64,
            4,
        ),
        observations: observed.len(),
    })
}

fn aggregate_rate<'a>(values: impl Iterator<Item = &'a ObservedRate>) -> Option<ObservedRate> {
    let values: Vec<&ObservedRate> = values.collect();
    let observations: usize = values.iter().map(|value| value.observations).sum();
    (observations > 0).then(|| ObservedRate {
        rate: values
            .iter()
            .map(|value| value.rate * value.observations as f64)
            .sum::<f64>()
            / observations as f64,
        observations,
    })
}

fn shrunk_rate(value: Option<&ObservedRate>) -> f64 {
    value.map_or(0.5, |value| {
        (value.rate * value.observations as f64 + 2.0) / (value.observations as f64 + 4.0)
    })
}

fn match_end(events: &[Value], clock: &EventClock) -> f64 {
    events
        .iter()
        .filter(|event| event_type(event) == Some("LogMatchEnd"))
        .filter_map(|event| clock.elapsed(event))
        .reduce(f64::max)
        .or_else(|| {
            events
                .iter()
                .filter_map(|event| clock.elapsed(event))
                .filter(|elapsed| *elapsed >= 0.0)
                .reduce(f64::max)
        })
        .unwrap_or_default()
        .max(0.0)
}

fn circle_to_public(circle: &CircleInternal) -> CircleState {
    CircleState {
        phase: circle.phase,
        center_x_m: round(circle.center.x / CM_PER_METER, 3),
        center_y_m: round(circle.center.y / CM_PER_METER, 3),
        radius_m: round(circle.radius / CM_PER_METER, 3),
        first_seen_s: round(circle.first_seen, 3),
        last_seen_s: round(circle.last_seen, 3),
        sample_count: circle.sample_count,
        source: circle.source.to_owned(),
    }
}

fn event_type(event: &Value) -> Option<&str> {
    event.get("_T").and_then(Value::as_str)
}

fn explicit_elapsed(event: &Value) -> Option<f64> {
    event
        .get("elapsedTime")
        .and_then(Value::as_f64)
        .or_else(|| {
            event
                .pointer("/gameState/elapsedTime")
                .and_then(Value::as_f64)
        })
}

fn timestamp_epoch(value: &Value) -> Option<f64> {
    DateTime::parse_from_rfc3339(value.as_str()?)
        .ok()
        .map(|timestamp| timestamp.timestamp_millis() as f64 / 1_000.0)
}

fn median(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let middle = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        Some((sorted[middle - 1] + sorted[middle]) / 2.0)
    } else {
        Some(sorted[middle])
    }
}

fn mean(values: &[f64]) -> Option<f64> {
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
}

fn mean_optional(values: impl Iterator<Item = Option<f64>>) -> Option<f64> {
    let values: Vec<f64> = values.flatten().collect();
    mean(&values)
}

fn weighted_optional(values: impl Iterator<Item = (f64, usize)>) -> Option<f64> {
    let values: Vec<(f64, usize)> = values.collect();
    let weight: usize = values.iter().map(|value| value.1).sum();
    (weight > 0).then(|| {
        values
            .iter()
            .map(|(value, count)| value * *count as f64)
            .sum::<f64>()
            / weight as f64
    })
}

fn percentile(values: &[f64], fraction: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    if sorted.len() == 1 {
        return sorted[0];
    }
    let index = fraction * (sorted.len() - 1) as f64;
    let lower = index.floor() as usize;
    let upper = (lower + 1).min(sorted.len() - 1);
    let weight = index - lower as f64;
    sorted[lower] * (1.0 - weight) + sorted[upper] * weight
}

fn round(value: f64, digits: i32) -> f64 {
    let multiplier = 10_f64.powi(digits);
    (value * multiplier).round() / multiplier
}

fn rounded_option(value: Option<f64>, digits: i32) -> Option<f64> {
    value.map(|value| round(value, digits))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::telemetry::decode_events;

    use super::{AnalysisConfig, analyze_dataset, analyze_match};

    fn fixture() -> Vec<serde_json::Value> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/minimal_telemetry.json");
        decode_events(&std::fs::read(path).expect("fixture should be readable"))
            .expect("fixture should parse")
    }

    #[test]
    fn produces_phase_aware_analysis_matching_reference_fixture() {
        let analysis = analyze_match("fixture", &fixture(), AnalysisConfig::default())
            .expect("analysis should complete");

        assert_eq!(analysis.map_name, "Baltic_Main");
        assert_eq!(analysis.observed_teams, 2);
        assert_eq!(analysis.observed_players, 3);
        assert_eq!(
            analysis
                .circles
                .iter()
                .map(|circle| circle.phase)
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert_eq!(analysis.team_snapshot_count, 8);
        assert_eq!(
            analysis
                .cells
                .iter()
                .map(|cell| cell.damage_dealt)
                .sum::<f64>(),
            25.0
        );
        assert_eq!(
            analysis
                .cells
                .iter()
                .map(|cell| cell.knocks_given)
                .sum::<usize>(),
            1
        );
        assert_eq!(
            analysis.cells.iter().map(|cell| cell.kills).sum::<usize>(),
            1
        );
    }

    #[test]
    fn aggregates_multiple_match_analyses_by_map_and_cell() {
        let analysis = analyze_match("fixture", &fixture(), AnalysisConfig::default())
            .expect("analysis should complete");
        let dataset = analyze_dataset(&[analysis.clone(), analysis]);

        assert_eq!(dataset.match_count, 2);
        assert_eq!(dataset.maps.len(), 1);
        assert_eq!(dataset.maps[0].match_count, 2);
        assert!(
            dataset.maps[0]
                .cells
                .iter()
                .all(|cell| cell.match_count == 2)
        );
    }
}
