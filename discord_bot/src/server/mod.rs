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
    match (&state.cloud_ssh_host, &state.cloud_ssh_key) {
        (Some(host), Some(key)) => {
            let ssh_args = [host, "-i", key, &format!("'{command} {}'", args.join(" "))];

            tracing::info!("executing: {} {}", &state.ssh_path, &ssh_args.join(" "));

            match tokio::process::Command::new(&state.ssh_path)
                .args(ssh_args)
                .output()
                .await {
                Ok(execution) => {
                    if !&execution.stderr.is_empty() {
                        tracing::error!("{}", std::str::from_utf8(&execution.stderr,)?);
                    }

                    Ok(std::str::from_utf8(&execution.stdout)?.to_string())
                },
                Err(e) => {
                    tracing::error!("{e}");
                    Err(AppError::Io(e))
                }
            }
        }
        _ => {
            tracing::info!("executing: {command} {}", &args.join(" "));

            let execution = tokio::process::Command::new(command)
                .args(args)
                .output()
                .await?;

            if !&execution.stderr.is_empty() {
                tracing::error!("{}", std::str::from_utf8(&execution.stderr,)?);
            }

            Ok(std::str::from_utf8(&execution.stdout)?.to_string())
        }
    }
}
