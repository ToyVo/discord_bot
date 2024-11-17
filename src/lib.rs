use crate::error::AppError;
#[cfg(feature = "db")]
use std::sync::LazyLock;
#[cfg(feature = "db")]
use surrealdb::engine::remote::ws::Client;
#[cfg(feature = "db")]
use surrealdb::Surreal;
use tokio::process::Command;

pub mod discord_utils;
pub mod error;
pub mod handlers;
pub mod minecraft;
pub mod models;
pub mod routes;
pub mod terraria;

#[cfg(feature = "db")]
pub static DB: LazyLock<Surreal<Client>> = LazyLock::new(Surreal::init);

#[cfg(target_os = "linux")]
pub async fn systemctl_running<S: AsRef<str>>(service_name: S) -> Result<bool, AppError> {
    let output = Command::new("systemctl")
        .args(["show", "-P", "ActiveState", service_name.as_ref()])
        .output()
        .await?;

    Ok(std::str::from_utf8(&output.stdout)?.trim() == "active")
}

#[cfg(target_os = "macos")]
pub async fn systemctl_running<S: AsRef<str>>(_service_name: S) -> Result<bool, AppError> {
    Ok(true)
}

pub async fn fs_sync() -> Result<(), AppError> {
    match Command::new("sync").output().await {
        Ok(output) => match output.status.code() {
            Some(0) => Ok(()),
            _ => Err(AppError::Other(String::from("Syncing filesystem failed"))),
        },
        Err(e) => Err(AppError::Other(format!("Syncing filesystem failed: {e}"))),
    }
}

pub async fn rclone(args: &[&str]) -> Result<String, AppError> {
    match Command::new("rclone").args(args).output().await {
        Ok(output) => {
            tracing::debug! {"rclone {}", output.status}
            let err = std::str::from_utf8(&output.stderr)?.trim();
            if !err.is_empty() {
                tracing::error!("rclone {}: {err}", args[0])
            }
            let out = std::str::from_utf8(&output.stdout)?.trim().to_string();
            if !out.is_empty() {
                tracing::info!("rclone {}: {out}", args[0])
            }
            Ok(out)
        }
        Err(err) => {
            tracing::error!("rclone {}: {err}", args[0]);
            Err(AppError::Other(String::from("rclone error")))
        }
    }
}
