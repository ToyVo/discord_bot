pub mod discord;
pub mod minecraft;
pub mod players;
mod state;
pub mod terraria;
use crate::error::AppError;
pub use state::*;

pub mod models;

pub async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

/// attempts to execute a command over ssh with local fallback
pub async fn ssh_command(
    command: &str,
    args: &[&str],
    state: &AppState,
) -> Result<String, AppError> {
    let logs = match (&state.cloud_ssh_host, &state.cloud_ssh_key) {
        (Some(host), Some(key)) => {
            let ssh_args = [host, "-i", key, &format!("'{command} {}'", args.join(" "))];
            std::str::from_utf8(
                &tokio::process::Command::new("ssh")
                    .args(ssh_args)
                    .output()
                    .await?
                    .stdout,
            )?
            .to_string()
        }
        _ => std::str::from_utf8(
            &tokio::process::Command::new(command)
                .args(args)
                .output()
                .await?
                .stdout,
        )?
        .to_string(),
    };
    Ok(logs)
}
