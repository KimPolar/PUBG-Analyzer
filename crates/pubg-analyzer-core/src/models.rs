use serde::{Deserialize, Serialize};

use crate::error::{AnalyzerError, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct AppSettings {
    pub username: String,
    pub platform: String,
    pub rpm: u32,
    pub download_concurrency: usize,
    pub cell_size_m: f64,
    pub bucket_seconds: f64,
    pub keep_raw_telemetry: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            username: String::new(),
            platform: "steam".to_owned(),
            rpm: 10,
            download_concurrency: 4,
            cell_size_m: 100.0,
            bucket_seconds: 10.0,
            keep_raw_telemetry: true,
        }
    }
}

impl AppSettings {
    pub fn validate(&self) -> Result<()> {
        if self.username.trim().is_empty() {
            return Err(AnalyzerError::InvalidConfiguration(
                "username is required".to_owned(),
            ));
        }
        if self.platform.is_empty()
            || !self
                .platform
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '-')
        {
            return Err(AnalyzerError::InvalidConfiguration(
                "platform must contain only letters, numbers, or hyphens".to_owned(),
            ));
        }
        if !(1..=600).contains(&self.rpm) {
            return Err(AnalyzerError::InvalidConfiguration(
                "rpm must be between 1 and 600".to_owned(),
            ));
        }
        if !(1..=16).contains(&self.download_concurrency) {
            return Err(AnalyzerError::InvalidConfiguration(
                "download concurrency must be between 1 and 16".to_owned(),
            ));
        }
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
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlayerProfile {
    pub account_id: String,
    pub name: String,
    pub platform: String,
    pub match_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MatchMetadata {
    pub match_id: String,
    pub platform: String,
    pub created_at: Option<String>,
    pub duration_seconds: Option<i64>,
    pub game_mode: Option<String>,
    pub map_name: Option<String>,
    pub match_type: Option<String>,
    pub patch_version: Option<String>,
    pub telemetry_url: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitState {
    pub limit: Option<u32>,
    pub remaining: Option<u32>,
    pub reset_unix: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "stage", rename_all = "camelCase")]
pub enum CollectionProgress {
    ResolvingPlayer,
    MatchesDiscovered { total: usize, fresh: usize },
    FetchingMatch { current: usize, total: usize },
    DownloadingTelemetry { current: usize, total: usize },
    Analyzing { current: usize, total: usize },
    MatchComplete { match_id: String },
    MatchFailed { match_id: String, message: String },
    Complete,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CollectionReport {
    pub discovered_matches: usize,
    pub new_matches: usize,
    pub analyzed_matches: usize,
    pub skipped_matches: usize,
    pub failed_matches: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DashboardData {
    pub player_name: Option<String>,
    pub platform: Option<String>,
    pub stored_matches: usize,
    pub analyzed_matches: usize,
    pub failed_matches: usize,
    pub maps: Vec<MapCount>,
    pub latest_matches: Vec<StoredMatchSummary>,
    pub dataset: Option<crate::analysis::DatasetAnalysis>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MapCount {
    pub map_name: String,
    pub matches: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StoredMatchSummary {
    pub match_id: String,
    pub created_at: Option<String>,
    pub map_name: Option<String>,
    pub game_mode: Option<String>,
    pub status: String,
    pub error: Option<String>,
}
