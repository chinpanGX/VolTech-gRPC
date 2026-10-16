use std::time::Duration;

use server::{BoxError, Config};
use tokio::net::TcpListener;
use voltech_grpc_log::RpcLogLayer;
use voltech_grpc_status::{FileStatusSource, ServerStatus};

// サーバーの状態の設定を読み直す間隔。再起動せずにメンテナンスを切り替えられる
const STATUS_RELOAD_INTERVAL: Duration = Duration::from_secs(5);

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    // .env が無くても既定値で動くので、読めなくても続ける
    let _ = dotenvy::dotenv();
    let config = Config::from_env()?;
    voltech_grpc_log::init(&config.log)?;

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
        RpcLogLayer::new(&config.log),
        config.game_log_sink,
        config.enable_reflection,
        shutdown_signal(),
    )
    .await
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
