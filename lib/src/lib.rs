use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[derive(Debug)]
pub enum AppError {
    Anyhow(anyhow::Error),
    EnvVar(std::env::VarError),
    Request(reqwest::Error),
    Json(serde_json::Error),
    Io(std::io::Error),
    Serenity(serenity::Error),
    Parse(url::ParseError),
    Other(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::Anyhow(e) => tracing::error!("Anyhow error: {:#}", e),
            AppError::EnvVar(e) => tracing::error!("Environment variable error: {:#}", e),
            AppError::Request(e) => tracing::error!("Request error: {:#}", e),
            AppError::Json(e) => tracing::error!("JSON error: {:#}", e),
            AppError::Io(e) => tracing::error!("IO error: {:#}", e),
            AppError::Serenity(e) => tracing::error!("Serenity error: {:#}", e),
            AppError::Parse(e) => tracing::error!("Parse error: {:#}", e),
            AppError::Other(e) => tracing::error!("Other error: {:#}", e),
        }

        (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong").into_response()
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Anyhow(e) => write!(f, "Anyhow error: {}", e),
            AppError::EnvVar(e) => write!(f, "Environment variable error: {}", e),
            AppError::Request(e) => write!(f, "Request error: {}", e),
            AppError::Json(e) => write!(f, "JSON error: {}", e),
            AppError::Io(e) => write!(f, "IO error: {}", e),
            AppError::Serenity(e) => write!(f, "Serenity error: {}", e),
            AppError::Parse(e) => write!(f, "Parse error: {}", e),
            AppError::Other(e) => write!(f, "Other error: {}", e),
        }
    }
}

impl std::error::Error for AppError {}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Anyhow(err)
    }
}

impl From<std::env::VarError> for AppError {
    fn from(err: std::env::VarError) -> Self {
        AppError::EnvVar(err)
    }
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        AppError::Request(err)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::Json(err)
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Io(err)
    }
}

impl From<serenity::Error> for AppError {
    fn from(err: serenity::Error) -> Self {
        AppError::Serenity(err)
    }
}

impl From<url::ParseError> for AppError {
    fn from(err: url::ParseError) -> Self {
        AppError::Parse(err)
    }
}
