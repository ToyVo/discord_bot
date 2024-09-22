use crate::routes::AppState;
use crate::error::AppError;

pub async fn track_players(state: &AppState) -> Result<(), AppError> {
    let mut server = <rcon::Connection<tokio::net::TcpStream>>::builder()
        .enable_minecraft_quirks(true)
        .connect(
            state.minecraft_rcon_address.as_str(),
            state.minecraft_rcon_password.as_str(),
        )
        .await?;

    let res = server
        .cmd("list")
        .await?;

    println!("Server Response: {res}");

    Ok(())
}
