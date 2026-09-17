use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
};

use rusqlite::{Connection, OptionalExtension, params};

use crate::{
    analysis::{MatchAnalysis, TrainingRow, analyze_dataset},
    error::Result,
    models::{
        AppSettings, DashboardData, MapCount, MatchMetadata, PlayerProfile, StoredMatchSummary,
    },
};

const SCHEMA_VERSION: i64 = 1;

#[derive(Debug, Clone)]
pub struct Database {
    path: Arc<PathBuf>,
}

impl Database {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let database = Self {
            path: Arc::new(path),
        };
        database.initialize()?;
        Ok(database)
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    fn connect(&self) -> Result<Connection> {
        let connection = Connection::open(self.path.as_path())?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        Ok(connection)
    }

    fn initialize(&self) -> Result<()> {
        let connection = self.connect()?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS schema_meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS settings (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                json TEXT NOT NULL,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS players (
                account_id TEXT PRIMARY KEY,
                platform TEXT NOT NULL,
                name TEXT NOT NULL,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS matches (
                match_id TEXT PRIMARY KEY,
                player_account_id TEXT NOT NULL,
                platform TEXT NOT NULL,
                created_at TEXT,
                duration_seconds INTEGER,
                game_mode TEXT,
                map_name TEXT,
                match_type TEXT,
                patch_version TEXT,
                telemetry_url TEXT NOT NULL,
                raw_path TEXT,
                raw_sha256 TEXT,
                status TEXT NOT NULL DEFAULT 'metadata',
                error TEXT,
                analysis_json TEXT,
                collected_at TEXT,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(player_account_id) REFERENCES players(account_id)
            );

            CREATE INDEX IF NOT EXISTS idx_matches_player
                ON matches(player_account_id, created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_matches_status
                ON matches(status);
            ",
        )?;
        connection.execute(
            "INSERT INTO schema_meta(key, value) VALUES('version', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [SCHEMA_VERSION.to_string()],
        )?;
        Ok(())
    }

    pub fn save_settings(&self, settings: &AppSettings) -> Result<()> {
        settings.validate()?;
        let json = serde_json::to_string(settings)?;
        self.connect()?.execute(
            "INSERT INTO settings(id, json, updated_at)
             VALUES(1, ?1, CURRENT_TIMESTAMP)
             ON CONFLICT(id) DO UPDATE SET
                json = excluded.json,
                updated_at = CURRENT_TIMESTAMP",
            [json],
        )?;
        Ok(())
    }

    pub fn load_settings(&self) -> Result<AppSettings> {
        let json: Option<String> = self
            .connect()?
            .query_row("SELECT json FROM settings WHERE id = 1", [], |row| {
                row.get(0)
            })
            .optional()?;
        json.map_or_else(
            || Ok(AppSettings::default()),
            |value| Ok(serde_json::from_str(&value)?),
        )
    }

    pub fn upsert_player(&self, player: &PlayerProfile) -> Result<()> {
        self.connect()?.execute(
            "INSERT INTO players(account_id, platform, name, updated_at)
             VALUES(?1, ?2, ?3, CURRENT_TIMESTAMP)
             ON CONFLICT(account_id) DO UPDATE SET
                platform = excluded.platform,
                name = excluded.name,
                updated_at = CURRENT_TIMESTAMP",
            params![player.account_id, player.platform, player.name],
        )?;
        Ok(())
    }

    pub fn upsert_match(&self, account_id: &str, metadata: &MatchMetadata) -> Result<()> {
        self.connect()?.execute(
            "INSERT INTO matches(
                match_id, player_account_id, platform, created_at,
                duration_seconds, game_mode, map_name, match_type,
                patch_version, telemetry_url, status, updated_at
             ) VALUES(
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                'metadata', CURRENT_TIMESTAMP
             )
             ON CONFLICT(match_id) DO UPDATE SET
                player_account_id = excluded.player_account_id,
                platform = excluded.platform,
                created_at = excluded.created_at,
                duration_seconds = excluded.duration_seconds,
                game_mode = excluded.game_mode,
                map_name = excluded.map_name,
                match_type = excluded.match_type,
                patch_version = excluded.patch_version,
                telemetry_url = excluded.telemetry_url,
                updated_at = CURRENT_TIMESTAMP",
            params![
                metadata.match_id,
                account_id,
                metadata.platform,
                metadata.created_at,
                metadata.duration_seconds,
                metadata.game_mode,
                metadata.map_name,
                metadata.match_type,
                metadata.patch_version,
                metadata.telemetry_url,
            ],
        )?;
        Ok(())
    }

    pub fn existing_match_ids(&self) -> Result<HashSet<String>> {
        read_id_set(&self.connect()?, "SELECT match_id FROM matches")
    }

    pub fn analyzed_match_ids(&self) -> Result<HashSet<String>> {
        read_id_set(
            &self.connect()?,
            "SELECT match_id FROM matches WHERE status = 'analyzed'",
        )
    }

    pub fn mark_downloading(&self, match_id: &str) -> Result<()> {
        self.connect()?.execute(
            "UPDATE matches SET status = 'downloading', error = NULL,
             updated_at = CURRENT_TIMESTAMP WHERE match_id = ?1",
            [match_id],
        )?;
        Ok(())
    }

    pub fn save_analysis(
        &self,
        match_id: &str,
        raw_path: Option<&Path>,
        raw_sha256: &str,
        analysis: &MatchAnalysis,
    ) -> Result<()> {
        let analysis_json = serde_json::to_string(analysis)?;
        let raw_path = raw_path.map(|path| path.to_string_lossy().into_owned());
        self.connect()?.execute(
            "UPDATE matches SET
                raw_path = ?2,
                raw_sha256 = ?3,
                status = 'analyzed',
                error = NULL,
                analysis_json = ?4,
                collected_at = CURRENT_TIMESTAMP,
                updated_at = CURRENT_TIMESTAMP
             WHERE match_id = ?1",
            params![match_id, raw_path, raw_sha256, analysis_json],
        )?;
        Ok(())
    }

    pub fn mark_failed(&self, match_id: &str, message: &str) -> Result<()> {
        self.connect()?.execute(
            "UPDATE matches SET status = 'failed', error = ?2,
             updated_at = CURRENT_TIMESTAMP WHERE match_id = ?1",
            params![match_id, message.chars().take(2_000).collect::<String>()],
        )?;
        Ok(())
    }

    pub fn load_analyses(&self) -> Result<Vec<MatchAnalysis>> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT analysis_json FROM matches
             WHERE status = 'analyzed' AND analysis_json IS NOT NULL
             ORDER BY created_at",
        )?;
        let json_values = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        json_values
            .into_iter()
            .map(|value| Ok(serde_json::from_str(&value)?))
            .collect()
    }

    pub fn load_training_rows(&self) -> Result<Vec<TrainingRow>> {
        Ok(self
            .load_analyses()?
            .into_iter()
            .flat_map(|analysis| analysis.training_rows)
            .collect())
    }

    pub fn dashboard(&self) -> Result<DashboardData> {
        let connection = self.connect()?;
        let player = connection
            .query_row(
                "SELECT name, platform FROM players ORDER BY updated_at DESC LIMIT 1",
                [],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;

        let mut status_counts = HashMap::new();
        {
            let mut statement =
                connection.prepare("SELECT status, COUNT(*) FROM matches GROUP BY status")?;
            for result in statement.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })? {
                let (status, count) = result?;
                status_counts.insert(status, usize::try_from(count).unwrap_or_default());
            }
        }

        let maps = {
            let mut statement = connection.prepare(
                "SELECT COALESCE(map_name, 'Unknown'), COUNT(*) FROM matches
                 WHERE status = 'analyzed' GROUP BY map_name ORDER BY COUNT(*) DESC",
            )?;
            statement
                .query_map([], |row| {
                    Ok(MapCount {
                        map_name: row.get(0)?,
                        matches: usize::try_from(row.get::<_, i64>(1)?).unwrap_or_default(),
                    })
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };

        let latest_matches = {
            let mut statement = connection.prepare(
                "SELECT match_id, created_at, map_name, game_mode, status, error
                 FROM matches ORDER BY COALESCE(created_at, updated_at) DESC LIMIT 20",
            )?;
            statement
                .query_map([], |row| {
                    Ok(StoredMatchSummary {
                        match_id: row.get(0)?,
                        created_at: row.get(1)?,
                        map_name: row.get(2)?,
                        game_mode: row.get(3)?,
                        status: row.get(4)?,
                        error: row.get(5)?,
                    })
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        drop(connection);

        let analyses = self.load_analyses()?;
        let dataset = (!analyses.is_empty()).then(|| analyze_dataset(&analyses));
        let analyzed_matches = status_counts.get("analyzed").copied().unwrap_or_default();
        let failed_matches = status_counts.get("failed").copied().unwrap_or_default();
        let stored_matches = status_counts.values().sum();

        Ok(DashboardData {
            player_name: player.as_ref().map(|value| value.0.clone()),
            platform: player.map(|value| value.1),
            stored_matches,
            analyzed_matches,
            failed_matches,
            maps,
            latest_matches,
            dataset,
        })
    }
}

fn read_id_set(connection: &Connection, sql: &str) -> Result<HashSet<String>> {
    let mut statement = connection.prepare(sql)?;
    let ids = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<std::result::Result<HashSet<_>, _>>()?;
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::Database;
    use crate::{
        analysis::MatchAnalysis,
        models::{AppSettings, MatchMetadata, PlayerProfile},
    };

    #[test]
    fn persists_non_secret_settings() {
        let directory = tempdir().expect("temporary directory");
        let database = Database::open(directory.path().join("analyzer.sqlite3"))
            .expect("database should open");
        let settings = AppSettings {
            username: "PlayerOne".to_owned(),
            rpm: 9,
            ..AppSettings::default()
        };
        database
            .save_settings(&settings)
            .expect("settings should save");
        assert_eq!(
            database.load_settings().expect("settings should load"),
            settings
        );
    }

    #[test]
    fn stores_analyzed_match_and_builds_dashboard() {
        let directory = tempdir().expect("temporary directory");
        let database = Database::open(directory.path().join("analyzer.sqlite3"))
            .expect("database should open");
        let player = PlayerProfile {
            account_id: "account-1".to_owned(),
            name: "PlayerOne".to_owned(),
            platform: "steam".to_owned(),
            match_ids: vec!["match-1".to_owned()],
        };
        database.upsert_player(&player).expect("player should save");
        database
            .upsert_match(
                &player.account_id,
                &MatchMetadata {
                    match_id: "match-1".to_owned(),
                    platform: "steam".to_owned(),
                    created_at: Some("2026-01-01T00:00:00Z".to_owned()),
                    duration_seconds: Some(1_400),
                    game_mode: Some("squad-fpp".to_owned()),
                    map_name: Some("Baltic_Main".to_owned()),
                    match_type: Some("competitive".to_owned()),
                    patch_version: Some("40.1".to_owned()),
                    telemetry_url: "https://telemetry-cdn.pubg.com/test.json".to_owned(),
                },
            )
            .expect("metadata should save");
        database
            .save_analysis(
                "match-1",
                None,
                "abc123",
                &MatchAnalysis {
                    schema_version: "0.2.0".to_owned(),
                    match_id: "match-1".to_owned(),
                    map_name: "Baltic_Main".to_owned(),
                    event_count: 42,
                    duration_seconds: 1_400.0,
                    observed_players: 64,
                    observed_teams: 16,
                    team_snapshot_count: 10,
                    movement_segment_count: 8,
                    circles: Vec::new(),
                    cells: Vec::new(),
                    training_rows: Vec::new(),
                },
            )
            .expect("analysis should save");

        let dashboard = database.dashboard().expect("dashboard should build");
        assert_eq!(dashboard.player_name.as_deref(), Some("PlayerOne"));
        assert_eq!(dashboard.stored_matches, 1);
        assert_eq!(dashboard.analyzed_matches, 1);
        assert_eq!(dashboard.maps[0].map_name, "Baltic_Main");
        assert_eq!(dashboard.dataset.expect("dataset").match_count, 1);
    }
}
