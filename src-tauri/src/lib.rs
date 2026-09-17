use std::{
    fs::File,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

use keyring::Entry;
use pubg_analyzer_core::{
    AppSettings, CollectionProgress, CollectionReport, CollectionService, DashboardData, Database,
    ProgressSink, PubgClient,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};
use tauri_plugin_shell::ShellExt;
use tokio::sync::Mutex;

const KEYRING_SERVICE: &str = "dev.kimpolar.pubg-analyzer";
const KEYRING_USER: &str = "pubg-api-key";

struct AppState {
    database: Database,
    data_directory: PathBuf,
    active_collection: Mutex<Option<CollectionService>>,
    training: Mutex<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SettingsResponse {
    settings: AppSettings,
    api_key_configured: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveSettingsRequest {
    settings: AppSettings,
    api_key: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TrainingResult {
    report: Value,
    model_directory: String,
}

#[tauri::command]
async fn get_settings(state: State<'_, AppState>) -> Result<SettingsResponse, String> {
    let database = state.database.clone();
    let settings = tauri::async_runtime::spawn_blocking(move || database.load_settings())
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())?;
    Ok(SettingsResponse {
        settings,
        api_key_configured: read_api_key().is_ok_and(|key| !key.trim().is_empty()),
    })
}

#[tauri::command]
async fn save_settings(
    state: State<'_, AppState>,
    request: SaveSettingsRequest,
) -> Result<SettingsResponse, String> {
    request
        .settings
        .validate()
        .map_err(|error| error.to_string())?;
    if let Some(api_key) = request.api_key.as_deref() {
        write_api_key(api_key)?;
    }
    let database = state.database.clone();
    let settings = request.settings;
    let to_save = settings.clone();
    tauri::async_runtime::spawn_blocking(move || database.save_settings(&to_save))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())?;
    Ok(SettingsResponse {
        settings,
        api_key_configured: read_api_key().is_ok_and(|key| !key.trim().is_empty()),
    })
}

#[tauri::command]
async fn start_collection(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<CollectionReport, String> {
    let settings = state
        .database
        .load_settings()
        .map_err(|error| error.to_string())?;
    settings.validate().map_err(|error| error.to_string())?;
    let api_key = read_api_key().map_err(|error| {
        format!("PUBG API 키를 불러올 수 없습니다. 설정에서 키를 저장해 주세요: {error}")
    })?;
    let api = PubgClient::new(&api_key, &settings.platform, settings.rpm)
        .map_err(|error| error.to_string())?;
    let service = CollectionService::new(
        api,
        state.database.clone(),
        &state.data_directory,
        &settings,
    )
    .map_err(|error| error.to_string())?;

    {
        let mut active = state.active_collection.lock().await;
        if active.is_some() {
            return Err("이미 수집 작업이 실행 중입니다".to_owned());
        }
        *active = Some(service.clone());
    }

    let event_window = window.clone();
    let progress: ProgressSink = Arc::new(move |event: CollectionProgress| {
        let _ = event_window.emit("collection-progress", event);
    });
    let result = service.collect(&settings.username, progress).await;
    state.active_collection.lock().await.take();
    result.map_err(|error| error.to_string())
}

#[tauri::command]
async fn cancel_collection(state: State<'_, AppState>) -> Result<(), String> {
    if let Some(service) = state.active_collection.lock().await.as_ref() {
        service.cancel();
    }
    Ok(())
}

#[tauri::command]
async fn get_dashboard(state: State<'_, AppState>) -> Result<DashboardData, String> {
    let database = state.database.clone();
    tauri::async_runtime::spawn_blocking(move || database.dashboard())
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn train_models(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<TrainingResult, String> {
    {
        let mut training = state.training.lock().await;
        if *training {
            return Err("이미 학습 작업이 실행 중입니다".to_owned());
        }
        *training = true;
    }

    let result = run_training(&app, &state).await;
    *state.training.lock().await = false;
    result
}

async fn run_training(app: &AppHandle, state: &AppState) -> Result<TrainingResult, String> {
    let training_directory = state.data_directory.join("training");
    let model_directory = state.data_directory.join("models");
    tokio::fs::create_dir_all(&training_directory)
        .await
        .map_err(|error| error.to_string())?;
    tokio::fs::create_dir_all(&model_directory)
        .await
        .map_err(|error| error.to_string())?;
    let input_path = training_directory.join("training-rows.jsonl");
    let database = state.database.clone();
    let export_path = input_path.clone();
    tauri::async_runtime::spawn_blocking(move || export_training_rows(&database, &export_path))
        .await
        .map_err(|error| error.to_string())??;

    let input_argument = path_argument(&input_path)?;
    let output_argument = path_argument(&model_directory)?;
    let output = app
        .shell()
        .sidecar("pubg-trainer")
        .map_err(|error| error.to_string())?
        .args([
            "train",
            "--input",
            input_argument,
            "--output-dir",
            output_argument,
        ])
        .output()
        .await
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr);
        return Err(format!("학습기가 실패했습니다: {}", message.trim()));
    }
    let report = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("학습 결과를 읽지 못했습니다: {error}"))?;
    Ok(TrainingResult {
        report,
        model_directory: model_directory.to_string_lossy().into_owned(),
    })
}

fn export_training_rows(database: &Database, path: &Path) -> Result<(), String> {
    let rows = database
        .load_training_rows()
        .map_err(|error| error.to_string())?;
    if rows.is_empty() {
        return Err("학습할 분석 데이터가 없습니다".to_owned());
    }
    let file = File::create(path).map_err(|error| error.to_string())?;
    let mut writer = BufWriter::new(file);
    for row in rows {
        serde_json::to_writer(&mut writer, &row).map_err(|error| error.to_string())?;
        writer.write_all(b"\n").map_err(|error| error.to_string())?;
    }
    writer.flush().map_err(|error| error.to_string())
}

fn path_argument(path: &Path) -> Result<&str, String> {
    path.to_str()
        .ok_or_else(|| format!("경로를 UTF-8로 변환할 수 없습니다: {}", path.display()))
}

fn credential_entry() -> Result<Entry, String> {
    Entry::new(KEYRING_SERVICE, KEYRING_USER).map_err(|error| error.to_string())
}

fn read_api_key() -> Result<String, String> {
    credential_entry()?
        .get_password()
        .map_err(|error| error.to_string())
}

fn write_api_key(api_key: &str) -> Result<(), String> {
    let entry = credential_entry()?;
    if api_key.trim().is_empty() {
        let _ = entry.delete_credential();
        return Ok(());
    }
    entry
        .set_password(api_key.trim())
        .map_err(|error| error.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "pubg_analyzer=info".into()),
        )
        .try_init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let data_directory = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_directory)?;
            let database = Database::open(data_directory.join("pubg-analyzer.sqlite3"))?;
            app.manage(AppState {
                database,
                data_directory,
                active_collection: Mutex::new(None),
                training: Mutex::new(false),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            start_collection,
            cancel_collection,
            get_dashboard,
            train_models
        ])
        .run(tauri::generate_context!())
        .expect("failed to run PUBG Analyzer");
}
