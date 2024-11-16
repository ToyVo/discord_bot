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
    let output = Command::new("sync")
        .output()
        .await?;
    match output.status.code() {
        Some(0) => Ok(()),
        _ => Err(AppError::Other(String::from("Syncing filesystem failed"))),
    }
}