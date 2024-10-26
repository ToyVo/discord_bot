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
        .args(["show", service_name.as_ref()])
        .output()
        .await?;

    let output_str = std::str::from_utf8(&output.stdout)?;
    match output_str
        .lines()
        .find(|line| line.starts_with("ActiveState=")) {
        Some(state) => {
            match state.split('=').last().unwrap() {
                "active" => Ok(true),
                _ => Ok(false),
            }
        }
        None => Err(AppError::Other(String::from("Could not find ActiveState in systemctl output")))
    }
}
