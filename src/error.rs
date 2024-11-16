use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[derive(Debug)]
pub enum AppError {
    Anyhow(anyhow::Error),
    Cron(croner::errors::CronError),
    EnvVar(std::env::VarError),
    Io(std::io::Error),
    Json(serde_json::Error),
    Other(String),
    Parse(url::ParseError),
    Rcon(rcon::Error),
    Request(reqwest::Error),
    Serenity(serenity::Error),
    Surreal(surrealdb::Error),
    Toml(toml::de::Error),
    UTF8(std::str::Utf8Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::Anyhow(e) => tracing::error!("Anyhow error: {e:}"),
            AppError::Cron(e) => tracing::error!("Cron error: {e:}"),
            AppError::EnvVar(e) => tracing::error!("Environment variable error: {e}"),
            AppError::Io(e) => tracing::error!("IO error: {e:}"),
            AppError::Json(e) => tracing::error!("JSON error: {e:}"),
            AppError::Other(e) => tracing::error!("Other error: {e:}"),
            AppError::Parse(e) => tracing::error!("Parse error: {e:}"),
            AppError::Rcon(e) => tracing::error!("RCON error: {e:}"),
            AppError::Request(e) => tracing::error!("Request error: {e:}"),
            AppError::Serenity(e) => tracing::error!("Serenity error: {e:}"),
            AppError::Surreal(e) => tracing::error!("Surreal error: {e:}"),
            AppError::Toml(e) => tracing::error!("TOML error: {e:}"),
            AppError::UTF8(e) => tracing::error!("UTF-8 error: {e:}"),
        }

        (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong").into_response()
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Anyhow(e) => write!(f, "Anyhow error: {}", e),
            AppError::Cron(e) => write!(f, "Cron error: {}", e),
            AppError::EnvVar(e) => write!(f, "Environment variable error: {}", e),
            AppError::Io(e) => write!(f, "IO error: {}", e),
            AppError::Json(e) => write!(f, "JSON error: {}", e),
            AppError::Other(e) => write!(f, "Other error: {}", e),
            AppError::Parse(e) => write!(f, "Parse error: {}", e),
            AppError::Rcon(e) => write!(f, "RCON error: {}", e),
            AppError::Request(e) => write!(f, "Request error: {}", e),
            AppError::Serenity(e) => write!(f, "Serenity error: {}", e),
            AppError::Surreal(e) => write!(f, "Surreal error: {}", e),
            AppError::Toml(e) => write!(f, "TOML error: {}", e),
            AppError::UTF8(e) => write!(f, "UTF-8 error: {}", e),
        }
    }
}

impl std::error::Error for AppError {}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        Self::Anyhow(err)
    }
}

impl From<croner::errors::CronError> for AppError {
    fn from(err: croner::errors::CronError) -> Self {
        Self::Cron(err)
    }
}

impl From<std::env::VarError> for AppError {
    fn from(err: std::env::VarError) -> Self {
        Self::EnvVar(err)
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err)
    }
}

impl From<url::ParseError> for AppError {
    fn from(err: url::ParseError) -> Self {
        Self::Parse(err)
    }
}

impl From<rcon::Error> for AppError {
    fn from(err: rcon::Error) -> Self {
        Self::Rcon(err)
    }
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        Self::Request(err)
    }
}

impl From<serenity::Error> for AppError {
    fn from(err: serenity::Error) -> Self {
        Self::Serenity(err)
    }
}

impl From<surrealdb::Error> for AppError {
    fn from(err: surrealdb::Error) -> Self {
        Self::Surreal(err)
    }
}

impl From<toml::de::Error> for AppError {
    fn from(err: toml::de::Error) -> Self {
        Self::Toml(err)
    }
}

impl From<std::str::Utf8Error> for AppError {
    fn from(err: std::str::Utf8Error) -> Self {
        Self::UTF8(err)
    }
}
