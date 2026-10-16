//! テストで使うサーバーの起動と、API を呼ぶ処理。

#![allow(
    dead_code,
    reason = "テストのファイルごとに使う関数が違い、使わないファイルでは dead_code になるため"
)]

use std::time::Duration;

use sqlx::MySqlPool;
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::Request;
use tonic::transport::{Channel, Server};
use tonic_health::pb::HealthCheckRequest;
use tonic_health::pb::health_client::HealthClient;
use tower::Layer;
use voltech_grpc_auth::proto::voltech::v1::device_service_client::DeviceServiceClient;
use voltech_grpc_auth::proto::voltech::v1::{IssueTokenRequest, RegisterRequest, RegisterResponse};
use voltech_grpc_auth::{
    AuthLayer, BoxError, DeviceServiceImpl, PlayerId, PlayerResolver, PlayerStatus,
};
use voltech_grpc_log::{LogConfig, RpcLogLayer};

const DEFAULT_DATABASE_URL: &str = "mysql://app:app@localhost:13306/app";
/// テストで登録に使う secret_key。
pub const SECRET_KEY: &str = "0123456789abcdef0123456789abcdef";

/// 常に同じプレイヤーの状態を返す PlayerResolver。
#[derive(Clone)]
struct FixedResolver(PlayerStatus);

impl PlayerResolver for FixedResolver {
    async fn resolve(&self, _player_id: PlayerId) -> Result<PlayerStatus, BoxError> {
        Ok(self.0)
    }
}

/// テスト用に起動した gRPC サーバーと、そこにつないだチャンネル。
pub struct TestServer {
    channel: Channel,
}

impl TestServer {
    /// DeviceService と、AuthLayer を付けたヘルスチェックのサービスを、本番と同じく RpcLogLayer を外側に付けて起動して接続する。
    pub async fn start(player_status: PlayerStatus, token_ttl: Duration) -> Self {
        let database_url =
            std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_owned());
        let pool = MySqlPool::connect(&database_url).await.unwrap();
        voltech_grpc_auth::migrate(&pool).await.unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        // パッケージには認証が必要なサービスが無いので、ヘルスチェックのサービスに AuthLayer を付けて代わりにする
        let (_, health) = tonic_health::server::health_reporter();
        let device = DeviceServiceImpl::new(pool.clone())
            .with_token_ttl(token_ttl)
            .into_server();
        let protected = AuthLayer::new(pool, FixedResolver(player_status)).layer(health);
        // ヘルスチェックを認証が必要なサービスの代わりにするので、アクセスログから除かない
        let mut log_config = LogConfig::new("test-server", "test");
        log_config.excluded_rpcs.clear();
        tokio::spawn(
            Server::builder()
                .layer(RpcLogLayer::new(&log_config))
                .add_service(device)
                .add_service(protected)
                .serve_with_incoming(TcpListenerStream::new(listener)),
        );

        let channel = Channel::from_shared(format!("http://{addr}"))
            .unwrap()
            .connect()
            .await
            .unwrap();
        Self { channel }
    }

    /// DeviceService のクライアントを作る。
    pub fn device(&self) -> DeviceServiceClient<Channel> {
        DeviceServiceClient::new(self.channel.clone())
    }

    /// SECRET_KEY でデバイスを登録し、device_id を返す。
    pub async fn register(&self) -> String {
        self.register_with(SECRET_KEY).await.unwrap()
    }

    /// 指定した secret_key でデバイスを登録し、device_id を返す。
    pub async fn register_with(&self, secret_key: &str) -> Result<String, tonic::Status> {
        self.register_response(secret_key)
            .await
            .map(|response| response.device_id)
    }

    /// 指定した secret_key でデバイスを登録し、レスポンス(device_id と player_id)を返す。
    pub async fn register_response(
        &self,
        secret_key: &str,
    ) -> Result<RegisterResponse, tonic::Status> {
        self.device()
            .register(RegisterRequest {
                secret_key: secret_key.to_owned(),
            })
            .await
            .map(|response| response.into_inner())
    }

    /// トークンを発行し、access_token を返す。
    pub async fn issue_token(
        &self,
        device_id: &str,
        secret_key: &str,
    ) -> Result<String, tonic::Status> {
        self.device()
            .issue_token(IssueTokenRequest {
                device_id: device_id.to_owned(),
                secret_key: secret_key.to_owned(),
            })
            .await
            .map(|response| response.into_inner().access_token)
    }

    /// `Bearer <token>` を付けて、認証が必要なサービスを呼ぶ。`None` なら何も付けない。
    pub async fn call_protected(&self, token: Option<&str>) -> Result<(), tonic::Status> {
        let authorization = token.map(|token| format!("Bearer {token}"));
        self.call_with_authorization(authorization.as_deref()).await
    }

    /// メタデータ `authorization` にそのままの値を付けて、認証が必要なサービスを呼ぶ。
    pub async fn call_with_authorization(
        &self,
        authorization: Option<&str>,
    ) -> Result<(), tonic::Status> {
        let mut request = Request::new(HealthCheckRequest::default());
        if let Some(authorization) = authorization {
            request
                .metadata_mut()
                .insert("authorization", authorization.parse().unwrap());
        }
        HealthClient::new(self.channel.clone())
            .check(request)
            .await
            .map(|_| ())
    }
}

/// プレイヤーが有効・トークンの有効期限が 1 時間のサーバーを起動する。
pub async fn start() -> TestServer {
    TestServer::start(PlayerStatus::Active, Duration::from_secs(3600)).await
}
