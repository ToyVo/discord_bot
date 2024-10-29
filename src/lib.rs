use tokio::process::Command;
use crate::error::AppError;

pub mod discord_utils;
pub mod error;
pub mod handlers;
pub mod minecraft;
pub mod models;
pub mod routes;
pub mod terraria;

pub async fn systemctl_running<S: AsRef<str>>(service_name: S) -> Result<bool, AppError> {
    let output = Command::new("systemctl")
        .args(["show", "-P", "ActiveState", service_name.as_ref()])
        .output()
        .await?;

    Ok(std::str::from_utf8(&output.stdout)?.trim() == "active")
}
