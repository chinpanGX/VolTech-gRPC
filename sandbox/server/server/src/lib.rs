mod config;
mod players;
mod sandbox_service;

use std::future::Future;

use sqlx::MySqlPool;
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::Server;
use tower::Layer;
use tower_http::trace::TraceLayer;
use voltech_grpc_auth::{AuthLayer, DeviceServiceImpl};
use voltech_grpc_status::{DateChangeLayer, ServerStatus, ServerStatusLayer};
use voltech_grpc_transaction::{IdempotencyLevels, RpcMethodLayer, Transactions};

pub use config::Config;
pub use players::Players;
pub use sandbox_service::SandboxServiceImpl;

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// DB に接続し、基盤と sandbox のマイグレーションを実行する。
pub async fn connect_and_migrate(database_url: &str) -> Result<MySqlPool, BoxError> {
    let pool = MySqlPool::connect(database_url).await?;
    voltech_grpc_auth::migrate(&pool).await?;
    voltech_grpc_transaction::migrate(&pool).await?;
    sqlx::migrate!("../migrations").run(&pool).await?;
    Ok(pool)
}

/// `listener` で受け付けた接続で gRPC サーバーを動かす。`shutdown` が完了すると止まる。
/// 基盤の Layer の組み方は、アプリ(samples)と同じにする。
pub async fn serve(
    listener: TcpListener,
    pool: MySqlPool,
    status: ServerStatus,
    enable_reflection: bool,
    shutdown: impl Future<Output = ()>,
) -> Result<(), BoxError> {
    let reflection = if enable_reflection {
        Some(
            tonic_reflection::server::Builder::configure()
                .register_encoded_file_descriptor_set(voltech_grpc_auth::FILE_DESCRIPTOR_SET)
                .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
                .build_v1()?,
        )
    } else {
        None
    };

    // 冪等性の対象かどうかは、sandbox の .proto の idempotency_level から決める
    let transactions = Transactions::new(
        pool.clone(),
        IdempotencyLevels::from_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)?,
    );
    let auth = AuthLayer::new(pool.clone(), Players::new(pool.clone()));
    let date_change = DateChangeLayer::new(status.clone());

    Server::builder()
        .layer(TraceLayer::new_for_grpc())
        // 強制アップデートとメンテナンスは、認証より先に全サービスで判定する
        .layer(ServerStatusLayer::new(status))
        // Transactions::run が RPC の名前を読めるように、全サービスの外側に付ける
        .layer(RpcMethodLayer)
        .add_service(DeviceServiceImpl::new(pool.clone()).into_server())
        // 日付変更は認証の後に判定するので、AuthLayer の内側に DateChangeLayer を付ける
        .add_service(
            auth.layer(date_change.layer(SandboxServiceImpl::new(transactions).into_server())),
        )
        .add_optional_service(reflection)
        .serve_with_incoming_shutdown(TcpListenerStream::new(listener), shutdown)
        .await?;
    Ok(())
}
