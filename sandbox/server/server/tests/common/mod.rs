//! テストで使うサーバーの起動と、API を呼ぶ処理。

// expect にしないのは、全部の関数を使うテストのファイルでは警告が出ず、expect のほうが警告になるため
#![allow(
    dead_code,
    reason = "テストのファイルごとに使う関数が違い、使わないファイルでは dead_code になるため"
)]

use proto::sandbox::v1::gacha_service_client::GachaServiceClient;
use proto::sandbox::v1::player_service_client::PlayerServiceClient;
use proto::sandbox::v1::sandbox_service_client::SandboxServiceClient;
use proto::sandbox::v1::{
    DrawRequest, DrawResponse, Player, SignInRequest, SignInResponse, SignUpRequest,
};
use sqlx::MySqlPool;
use tokio::net::TcpListener;
use tonic::Request;
use tonic::transport::Channel;
use ulid::Ulid;
use voltech_grpc_auth::proto::voltech::v1::device_service_client::DeviceServiceClient;
use voltech_grpc_auth::proto::voltech::v1::{IssueTokenRequest, RegisterRequest, RegisterResponse};
use voltech_grpc_log::RpcLogLayer;
use voltech_grpc_status::{FileStatusSource, ServerStatus, StatusSource};
use voltech_grpc_transaction::GameLogSink;

const DEFAULT_DATABASE_URL: &str = "mysql://app:app@localhost:13306/voltech_sandbox";

/// テストで登録に使う secret_key。
pub const SECRET_KEY: &str = "0123456789abcdef0123456789abcdef";

/// テスト用に起動したサーバーと、そこにつないだチャンネル・DB。
pub struct TestServer {
    pub pool: MySqlPool,
    channel: Channel,
}

impl TestServer {
    /// マイグレーションを実行し、本番と同じルーターと、リポジトリの server_status.toml でサーバーを起動して接続する。
    pub async fn start() -> Self {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../config/server_status.toml");
        let config = FileStatusSource::new(path).load().await.unwrap();
        Self::start_with_status(ServerStatus::new(config)).await
    }

    /// `status` のサーバーの状態で起動する(メンテナンスなどを試すとき)。
    pub async fn start_with_status(status: ServerStatus) -> Self {
        let database_url = std::env::var("SANDBOX_DATABASE_URL")
            .unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_owned());
        let pool = server::connect_and_migrate(&database_url).await.unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(server::serve(
            listener,
            pool.clone(),
            status,
            RpcLogLayer::default(),
            GameLogSink::Database,
            true,
            std::future::pending(),
        ));

        let channel = Channel::from_shared(format!("http://{addr}"))
            .unwrap()
            .connect()
            .await
            .unwrap();
        Self { pool, channel }
    }

    /// 接続したチャンネル。ヘッダーを自分で付けて呼ぶテストで使う。
    pub fn channel(&self) -> Channel {
        self.channel.clone()
    }

    /// DeviceService のクライアントを作る。
    pub fn device(&self) -> DeviceServiceClient<Channel> {
        DeviceServiceClient::new(self.channel.clone())
    }

    /// SECRET_KEY でデバイスを登録し、レスポンス(device_id と player_id)を返す。
    pub async fn register(&self) -> RegisterResponse {
        self.device()
            .register(RegisterRequest {
                secret_key: SECRET_KEY.to_owned(),
            })
            .await
            .unwrap()
            .into_inner()
    }

    /// デバイスを登録してトークンを発行し、(デバイス登録で発行された player_id, access_token) を返す。
    pub async fn register_and_issue_token(&self) -> (Ulid, String) {
        let registered = self.register().await;
        let token = self
            .device()
            .issue_token(IssueTokenRequest {
                device_id: registered.device_id,
                secret_key: SECRET_KEY.to_owned(),
            })
            .await
            .unwrap()
            .into_inner()
            .access_token;
        (Ulid::from_string(&registered.player_id).unwrap(), token)
    }

    /// `Bearer <token>` を付けてサインアップする。`None` なら何も付けない。
    pub async fn sign_up(&self, token: Option<&str>) -> Result<Player, tonic::Status> {
        PlayerServiceClient::new(self.channel.clone())
            .sign_up(with_token(SignUpRequest {}, token))
            .await
            .map(|response| response.into_inner().player.unwrap())
    }

    /// `Bearer <token>` を付けてサインインする。`None` なら何も付けない。
    pub async fn sign_in(&self, token: Option<&str>) -> Result<Player, tonic::Status> {
        PlayerServiceClient::new(self.channel.clone())
            .sign_in(with_token(SignInRequest {}, token))
            .await
            .map(|response| response.into_inner().player.unwrap())
    }

    /// `Bearer <token>` を付けてサインインし、レスポンスをそのまま返す(所持アイテムも見るとき)。
    pub async fn sign_in_response(&self, token: &str) -> SignInResponse {
        PlayerServiceClient::new(self.channel.clone())
            .sign_in(with_token(SignInRequest {}, Some(token)))
            .await
            .unwrap()
            .into_inner()
    }

    /// `Bearer <token>` と新しい `x-request-id` を付けてガチャを1回引く。
    pub async fn draw(&self, token: &str) -> Result<DrawResponse, tonic::Status> {
        self.draw_with_request_id(token, Some(&Ulid::generate().to_string()))
            .await
    }

    /// `Bearer <token>` と新しい `x-request-id` を付けて、1日1回の無料ガチャを引く。
    pub async fn draw_free(&self, token: &str) -> Result<DrawResponse, tonic::Status> {
        self.send_draw(
            DrawRequest { free: true },
            token,
            Some(&Ulid::generate().to_string()),
        )
        .await
    }

    /// `Bearer <token>` と、指定した `x-request-id`(`None` なら付けない)を付けてガチャを1回引く。
    pub async fn draw_with_request_id(
        &self,
        token: &str,
        request_id: Option<&str>,
    ) -> Result<DrawResponse, tonic::Status> {
        self.send_draw(DrawRequest { free: false }, token, request_id)
            .await
    }

    async fn send_draw(
        &self,
        message: DrawRequest,
        token: &str,
        request_id: Option<&str>,
    ) -> Result<DrawResponse, tonic::Status> {
        let mut request = with_token(message, Some(token));
        if let Some(request_id) = request_id {
            request
                .metadata_mut()
                .insert("x-request-id", request_id.parse().unwrap());
        }
        GachaServiceClient::new(self.channel.clone())
            .draw(request)
            .await
            .map(|response| response.into_inner())
    }

    /// SandboxService のクライアントを作る。リクエストは `request` で作る。
    pub fn sandbox(&self) -> SandboxServiceClient<Channel> {
        SandboxServiceClient::new(self.channel.clone())
    }

    /// プレイヤーのジェムを、DB を直接書き換えて変える。
    pub async fn set_gems(&self, player_id: Ulid, gems: i64) {
        sqlx::query("UPDATE players SET gems = ? WHERE player_id = ?")
            .bind(gems)
            .bind(player_id.to_bytes().as_slice())
            .execute(&self.pool)
            .await
            .unwrap();
    }
}

/// `Bearer <token>` と、`request_id`(`None` なら付けない)を付けたリクエストを作る。
pub fn request<T>(message: T, token: &str, request_id: Option<&str>) -> Request<T> {
    let mut request = with_token(message, Some(token));
    if let Some(request_id) = request_id {
        request
            .metadata_mut()
            .insert("x-request-id", request_id.parse().unwrap());
    }
    request
}

fn with_token<T>(message: T, token: Option<&str>) -> Request<T> {
    let mut request = Request::new(message);
    if let Some(token) = token {
        request
            .metadata_mut()
            .insert("authorization", format!("Bearer {token}").parse().unwrap());
    }
    request
}
