use std::time::Duration;

use server::{BoxError, Config};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;
use voltech_grpc_status::{FileStatusSource, ServerStatus};

// サーバーの状態の設定を読み直す間隔。再起動せずにメンテナンスを切り替えられる
const STATUS_RELOAD_INTERVAL: Duration = Duration::from_secs(5);

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    // .env が無くても既定値で動くので、読めなくても続ける
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = Config::from_env()?;
    // 一度も読めなければ起動しない
    let status = ServerStatus::watch(
        FileStatusSource::new(&config.server_status_path),
        STATUS_RELOAD_INTERVAL,
    )
    .await?;
    let pool = server::connect_and_migrate(&config.database_url).await?;
    let listener = TcpListener::bind(config.listen_addr).await?;
    tracing::info!(addr = %config.listen_addr, reflection = config.enable_reflection, "listening");

    server::serve(
        listener,
        pool,
        status,
        config.enable_reflection,
        shutdown_signal(),
    )
    .await
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
