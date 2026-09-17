use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

use flate2::{Compression, write::GzEncoder};
use futures_util::{StreamExt, stream};

use crate::{
    analysis::{AnalysisConfig, analyze_match},
    error::{AnalyzerError, Result},
    models::{AppSettings, CollectionProgress, CollectionReport, MatchMetadata},
    pubg_api::PubgClient,
    storage::Database,
    telemetry::{decode_events, payload_sha256},
};

pub type ProgressSink = Arc<dyn Fn(CollectionProgress) + Send + Sync + 'static>;

#[derive(Clone)]
pub struct CollectionService {
    api: PubgClient,
    database: Database,
    raw_directory: Arc<PathBuf>,
    analysis_config: AnalysisConfig,
    concurrency: usize,
    keep_raw_telemetry: bool,
    cancelled: Arc<AtomicBool>,
}

impl CollectionService {
    pub fn new(
        api: PubgClient,
        database: Database,
        data_directory: impl Into<PathBuf>,
        settings: &AppSettings,
    ) -> Result<Self> {
        settings.validate()?;
        Ok(Self {
            api,
            database,
            raw_directory: Arc::new(data_directory.into().join("telemetry")),
            analysis_config: AnalysisConfig {
                cell_size_m: settings.cell_size_m,
                bucket_seconds: settings.bucket_seconds,
            }
            .validate()?,
            concurrency: settings.download_concurrency,
            keep_raw_telemetry: settings.keep_raw_telemetry,
            cancelled: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub async fn collect(
        &self,
        username: &str,
        progress: ProgressSink,
    ) -> Result<CollectionReport> {
        self.cancelled.store(false, Ordering::Release);
        progress(CollectionProgress::ResolvingPlayer);
        self.check_cancelled()?;

        let player = self.api.lookup_player(username).await?;
        self.database.upsert_player(&player)?;

        let existing = self.database.existing_match_ids()?;
        let analyzed = self.database.analyzed_match_ids()?;
        let pending_ids: Vec<String> = player
            .match_ids
            .iter()
            .filter(|match_id| !analyzed.contains(*match_id))
            .cloned()
            .collect();
        let fresh = pending_ids
            .iter()
            .filter(|match_id| !existing.contains(*match_id))
            .count();
        progress(CollectionProgress::MatchesDiscovered {
            total: player.match_ids.len(),
            fresh,
        });

        let mut report = CollectionReport {
            discovered_matches: player.match_ids.len(),
            new_matches: fresh,
            skipped_matches: player.match_ids.len().saturating_sub(pending_ids.len()),
            ..CollectionReport::default()
        };
        if pending_ids.is_empty() {
            progress(CollectionProgress::Complete);
            return Ok(report);
        }

        let metadata_current = Arc::new(AtomicUsize::new(0));
        let metadata_total = pending_ids.len();
        let api = self.api.clone();
        let metadata_progress = Arc::clone(&progress);
        let cancelled = Arc::clone(&self.cancelled);
        let mut metadata_stream = stream::iter(pending_ids.into_iter().map(|match_id| {
            let api = api.clone();
            let progress = Arc::clone(&metadata_progress);
            let current = Arc::clone(&metadata_current);
            let cancelled = Arc::clone(&cancelled);
            async move {
                if cancelled.load(Ordering::Acquire) {
                    return (match_id, Err(AnalyzerError::Cancelled));
                }
                let result = api.fetch_match(&match_id).await;
                let completed = current.fetch_add(1, Ordering::AcqRel) + 1;
                progress(CollectionProgress::FetchingMatch {
                    current: completed,
                    total: metadata_total,
                });
                (match_id, result)
            }
        }))
        .buffer_unordered(self.concurrency);

        let mut metadata = Vec::new();
        while let Some((match_id, result)) = metadata_stream.next().await {
            match result {
                Ok(item) => {
                    self.database.upsert_match(&player.account_id, &item)?;
                    metadata.push(item);
                }
                Err(AnalyzerError::Cancelled) => return Err(AnalyzerError::Cancelled),
                Err(error) => {
                    report.failed_matches += 1;
                    progress(CollectionProgress::MatchFailed {
                        match_id,
                        message: error.to_string(),
                    });
                }
            }
        }

        std::fs::create_dir_all(self.raw_directory.as_path())?;
        let download_current = Arc::new(AtomicUsize::new(0));
        let analyze_current = Arc::new(AtomicUsize::new(0));
        let telemetry_total = metadata.len();
        let service = self.clone();
        let telemetry_progress = Arc::clone(&progress);
        let mut telemetry_stream = stream::iter(metadata.into_iter().map(|metadata| {
            let service = service.clone();
            let progress = Arc::clone(&telemetry_progress);
            let download_current = Arc::clone(&download_current);
            let analyze_current = Arc::clone(&analyze_current);
            async move {
                let match_id = metadata.match_id.clone();
                let downloaded = download_current.fetch_add(1, Ordering::AcqRel) + 1;
                progress(CollectionProgress::DownloadingTelemetry {
                    current: downloaded,
                    total: telemetry_total,
                });
                let result = service
                    .collect_match(
                        metadata,
                        Arc::clone(&progress),
                        Arc::clone(&analyze_current),
                        telemetry_total,
                    )
                    .await;
                (match_id, result)
            }
        }))
        .buffer_unordered(self.concurrency);

        while let Some((match_id, result)) = telemetry_stream.next().await {
            match result {
                Ok(()) => {
                    report.analyzed_matches += 1;
                    progress(CollectionProgress::MatchComplete { match_id });
                }
                Err(AnalyzerError::Cancelled) => return Err(AnalyzerError::Cancelled),
                Err(error) => {
                    report.failed_matches += 1;
                    let message = error.to_string();
                    let _ = self.database.mark_failed(&match_id, &message);
                    progress(CollectionProgress::MatchFailed { match_id, message });
                }
            }
        }

        progress(CollectionProgress::Complete);
        Ok(report)
    }

    async fn collect_match(
        &self,
        metadata: MatchMetadata,
        progress: ProgressSink,
        analyze_current: Arc<AtomicUsize>,
        total: usize,
    ) -> Result<()> {
        self.check_cancelled()?;
        self.database.mark_downloading(&metadata.match_id)?;
        let payload = self.api.fetch_telemetry(&metadata.telemetry_url).await?;
        self.check_cancelled()?;

        let analyzed = analyze_current.fetch_add(1, Ordering::AcqRel) + 1;
        progress(CollectionProgress::Analyzing {
            current: analyzed,
            total,
        });
        let match_id = metadata.match_id.clone();
        let config = self.analysis_config;
        let keep_raw = self.keep_raw_telemetry;
        let (analysis, compressed, sha256) = tokio::task::spawn_blocking(move || {
            let sha256 = payload_sha256(&payload);
            let events = decode_events(&payload)?;
            let analysis = analyze_match(&match_id, &events, config)?;
            let compressed = if keep_raw {
                Some(gzip_payload(&payload)?)
            } else {
                None
            };
            Ok::<_, AnalyzerError>((analysis, compressed, sha256))
        })
        .await
        .map_err(|error| AnalyzerError::Task(error.to_string()))??;
        self.check_cancelled()?;

        let raw_path = if let Some(compressed) = compressed {
            let path = self
                .raw_directory
                .join(format!("{}.json.gz", metadata.match_id));
            tokio::fs::write(&path, compressed).await?;
            Some(path)
        } else {
            None
        };
        self.database
            .save_analysis(&metadata.match_id, raw_path.as_deref(), &sha256, &analysis)?;
        Ok(())
    }

    fn check_cancelled(&self) -> Result<()> {
        if self.cancelled.load(Ordering::Acquire) {
            Err(AnalyzerError::Cancelled)
        } else {
            Ok(())
        }
    }
}

fn gzip_payload(payload: &[u8]) -> Result<Vec<u8>> {
    if payload.starts_with(&[0x1f, 0x8b]) {
        return Ok(payload.to_vec());
    }
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(payload)?;
    Ok(encoder.finish()?)
}

#[allow(dead_code)]
fn _assert_path_is_send_sync(_: &Path) {}
