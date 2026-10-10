//! 起動したサーバーに gRPC でつなぎ、基盤の機能(認証・エラー・冪等性)が sandbox から使えることを確かめる。

use proto::errors;
use proto::sandbox::v1::sandbox_service_client::SandboxServiceClient;
use proto::sandbox::v1::{
    BadgeType, FailRequest, IncrementRequest, IncrementResponse, PingRequest, SignUpRequest,
};
use tokio::net::TcpListener;
use tonic::transport::Channel;
use tonic::{Code, Request};
use tonic_types::StatusExt;
use ulid::Ulid;
use voltech_grpc_auth::proto::voltech::v1::device_service_client::DeviceServiceClient;
use voltech_grpc_auth::proto::voltech::v1::{IssueTokenRequest, RegisterRequest};
use voltech_grpc_error::{ErrorCode, reserved};
use voltech_grpc_status::{FileStatusSource, ServerStatus, StatusSource};

const DEFAULT_DATABASE_URL: &str = "mysql://app:app@localhost:13306/voltech_sandbox";
const SECRET_KEY: &str = "0123456789abcdef0123456789abcdef";

/// マイグレーションを実行し、本番と同じルーターと sandbox の server_status.toml でサーバーを起動して接続する。
async fn start() -> Channel {
    let database_url =
        std::env::var("SANDBOX_DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_owned());
    let pool = server::connect_and_migrate(&database_url).await.unwrap();
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../config/server_status.toml");
    let status = ServerStatus::new(FileStatusSource::new(path).load().await.unwrap());

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(server::serve(
        listener,
        pool,
        status,
        false,
        std::future::pending(),
    ));
    Channel::from_shared(format!("http://{addr}"))
        .unwrap()
        .connect()
        .await
        .unwrap()
}

/// デバイスを登録してトークンを発行し、(player_id, access_token) を返す。
async fn register(channel: &Channel) -> (String, String) {
    let mut device = DeviceServiceClient::new(channel.clone());
    let registered = device
        .register(RegisterRequest {
            secret_key: SECRET_KEY.to_owned(),
        })
        .await
        .unwrap()
        .into_inner();
    let token = device
        .issue_token(IssueTokenRequest {
            device_id: registered.device_id,
            secret_key: SECRET_KEY.to_owned(),
        })
        .await
        .unwrap()
        .into_inner()
        .access_token;
    (registered.player_id, token)
}

/// `Bearer <token>` と、`request_id`(`None` なら付けない)を付けたリクエストを作る。
fn request<T>(message: T, token: &str, request_id: Option<&str>) -> Request<T> {
    let mut request = Request::new(message);
    request
        .metadata_mut()
        .insert("authorization", format!("Bearer {token}").parse().unwrap());
    if let Some(request_id) = request_id {
        request
            .metadata_mut()
            .insert("x-request-id", request_id.parse().unwrap());
    }
    request
}

async fn increment(
    client: &mut SandboxServiceClient<Channel>,
    token: &str,
    request_id: &str,
    amount: i64,
) -> IncrementResponse {
    client
        .increment(request(
            IncrementRequest { amount },
            token,
            Some(request_id),
        ))
        .await
        .unwrap()
        .into_inner()
}

/// サインアップの前の Ping は PLAYER_NOT_FOUND になり、サインアップ後は共通レスポンス項目とサーバー時刻のヘッダー付きで返る。
#[tokio::test]
async fn ping_after_sign_up_returns_common_response_fields() {
    let channel = start().await;
    let (player_id, token) = register(&channel).await;
    let mut client = SandboxServiceClient::new(channel);

    let status = client
        .ping(request(PingRequest {}, &token, None))
        .await
        .unwrap_err();
    assert_eq!(status.code(), Code::FailedPrecondition);
    let info = status.get_details_error_info().unwrap();
    assert_eq!(info.domain, errors::DOMAIN);
    assert_eq!(info.reason, errors::PLAYER_NOT_FOUND.reason());

    client
        .sign_up(request(SignUpRequest {}, &token, None))
        .await
        .unwrap();
    let response = client
        .ping(request(PingRequest {}, &token, None))
        .await
        .unwrap();
    assert!(response.metadata().get("x-server-time").is_some());
    assert!(response.metadata().get("x-game-date").is_some());
    let ping = response.into_inner();
    assert_eq!(ping.player_id, player_id);
    assert_eq!(ping.player_diff.unwrap().counter, Some(0));
    assert_eq!(ping.badges.len(), 1);
    assert_eq!(ping.badges[0].r#type(), BadgeType::Counter);
    assert!(!ping.badges[0].visible);
    assert_eq!(ping.notifications.len(), 1);
}

/// Fail は指定したエラーコードの FAILED_PRECONDITION で失敗する。指定が無ければ INTERNAL_ERROR、知らないコードなら INVALID_ARGUMENT。
#[tokio::test]
async fn fail_returns_requested_error_code() {
    let channel = start().await;
    let (_, token) = register(&channel).await;
    let mut client = SandboxServiceClient::new(channel);

    for (requested, expected) in [
        (errors::SANDBOX_STAY, errors::SANDBOX_STAY),
        (reserved::MAINTENANCE, reserved::MAINTENANCE),
        (ErrorCode::new("", ""), reserved::INTERNAL_ERROR),
    ] {
        let status = client
            .fail(request(fail_request(requested), &token, None))
            .await
            .unwrap_err();
        assert_eq!(status.code(), Code::FailedPrecondition);
        let info = status.get_details_error_info().unwrap();
        assert_eq!(info.domain, expected.domain());
        assert_eq!(info.reason, expected.reason());
    }

    // 基盤の予約エラーの reason でも、domain が違えば別のエラー
    let status = client
        .fail(request(
            fail_request(ErrorCode::new(errors::DOMAIN, "MAINTENANCE")),
            &token,
            None,
        ))
        .await
        .unwrap_err();
    assert_eq!(status.code(), Code::InvalidArgument);
}

fn fail_request(code: ErrorCode) -> FailRequest {
    FailRequest {
        domain: code.domain().to_owned(),
        reason: code.reason().to_owned(),
    }
}

/// Increment を同じ RequestID で2回呼ぶと、2回目は保存済みのレスポンスが返り、1回分だけ足される。
#[tokio::test]
async fn increment_with_same_request_id_is_applied_once() {
    let channel = start().await;
    let (_, token) = register(&channel).await;
    let mut client = SandboxServiceClient::new(channel);
    client
        .sign_up(request(SignUpRequest {}, &token, None))
        .await
        .unwrap();

    let request_id = Ulid::generate().to_string();
    let first = increment(&mut client, &token, &request_id, 5).await;
    let resent = increment(&mut client, &token, &request_id, 5).await;
    assert_eq!(first.counter, 5);
    assert_eq!(resent, first);

    let next = increment(&mut client, &token, &Ulid::generate().to_string(), 5).await;
    assert_eq!(next.counter, 10);
}

/// 冪等性の対象の Increment は、x-request-id が無いと失敗する。
#[tokio::test]
async fn increment_without_request_id_is_rejected() {
    let channel = start().await;
    let (_, token) = register(&channel).await;
    let mut client = SandboxServiceClient::new(channel);
    client
        .sign_up(request(SignUpRequest {}, &token, None))
        .await
        .unwrap();

    let status = client
        .increment(request(IncrementRequest { amount: 1 }, &token, None))
        .await
        .unwrap_err();
    assert_ne!(status.code(), Code::Ok);
}
