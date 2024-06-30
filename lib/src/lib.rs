#[derive(Debug)]
pub enum AppError {
    EnvVar(std::env::VarError),
    Request(reqwest::Error),
    Json(serde_json::Error),
    Io(std::io::Error),
    Other(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::EnvVar(e) => write!(f, "Environment variable error: {}", e),
            AppError::Request(e) => write!(f, "Request error: {}", e),
            AppError::Json(e) => write!(f, "JSON error: {}", e),
            AppError::Io(e) => write!(f, "IO error: {}", e),
            AppError::Other(e) => write!(f, "Other error: {}", e),
        }
    }
}

impl std::error::Error for AppError {}

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

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
