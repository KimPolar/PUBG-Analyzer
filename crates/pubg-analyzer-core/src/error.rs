use thiserror::Error;

#[derive(Debug, Error)]
pub enum AnalyzerError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("PUBG API returned HTTP {status}: {message}")]
    ApiStatus { status: u16, message: String },

    #[error("JSON processing failed: {0}")]
    Json(#[from] serde_json::Error),

    #[error("I/O operation failed: {0}")]
    Io(#[from] std::io::Error),

    #[error("database operation failed: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("invalid telemetry: {0}")]
    InvalidTelemetry(String),

    #[error("invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("match {0} did not contain a telemetry asset")]
    MissingTelemetry(String),

    #[error("operation was cancelled")]
    Cancelled,

    #[error("background task failed: {0}")]
    Task(String),
}

pub type Result<T> = std::result::Result<T, AnalyzerError>;
