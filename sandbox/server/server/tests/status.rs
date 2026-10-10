mod common;

use jiff::Timestamp;
use proto::sandbox::v1::SignInRequest;
use proto::sandbox::v1::player_service_client::PlayerServiceClient;
use tonic::{Code, Request};
use tonic_types::StatusExt;
use voltech_grpc_auth::proto::voltech::v1::IssueTokenRequest;
use voltech_grpc_error::reserved;
use voltech_grpc_status::{ServerStatus, StatusConfig};

use common::{SECRET_KEY, TestServer};

const CONFIG: &str = r#"
[maintenance]
enabled = false

[min_version]
ios = 0
android = 0
windows = 100

[game_day]
reset_hour = 4
timezone = "Asia/Tokyo"
"#;

fn status_with(config: &str) -> ServerStatus {
    ServerStatus::new(StatusConfig::from_toml(config).unwrap())
}

fn assert_reason(status: &tonic::Status, reason: &str) {
    assert_eq!(status.code(), Code::FailedPrecondition, "{status:?}");
    assert_eq!(status.get_details_error_info().unwrap().reason, reason);
}

/// メンテナンス中は、認証の要らないトークンの発行でも MAINTENANCE と終了予定が返る(専用の API は要らない)。
#[tokio::test]
async fn maintenance_blocks_issue_token() {
    let server = TestServer::start().await;
    let registered = server.register().await;

    let config = CONFIG.replace(
        "enabled = false",
        "enabled = true\nend_at = \"2026-10-06T15:00:00+09:00\"",
    );
    let maintenance = TestServer::start_with_status(status_with(&config)).await;
    let status = maintenance
        .device()
        .issue_token(IssueTokenRequest {
            device_id: registered.device_id,
            secret_key: SECRET_KEY.to_owned(),
        })
        .await
        .unwrap_err();

    assert_reason(&status, reserved::MAINTENANCE.reason());
    let info = status.get_details_error_info().unwrap();
    assert_eq!(info.metadata["end_at"], "2026-10-06T06:00:00Z");
}

/// 最低バージョンより古いアプリには、認証の前に UPDATE_REQUIRED が返る(トークンが無くても)。
#[tokio::test]
async fn old_build_gets_update_required_before_auth() {
    let server = TestServer::start_with_status(status_with(CONFIG)).await;

    let mut request = Request::new(SignInRequest {});
    request
        .metadata_mut()
        .insert("x-app-platform", "windows".parse().unwrap());
    request
        .metadata_mut()
        .insert("x-app-version", "99".parse().unwrap());
    let status = PlayerServiceClient::new(server.channel())
        .sign_in(request)
        .await
        .unwrap_err();

    assert_reason(&status, reserved::UPDATE_REQUIRED.reason());
}

/// 前の日付を付けたリクエストには DATE_CHANGED が返る。日付を付けないサインインは通り、
/// レスポンスのヘッダーでサーバー時刻と今のゲーム内の日付を受け取れる。
#[tokio::test]
async fn date_changed_after_auth_and_response_headers() {
    let status = status_with(CONFIG);
    let server = TestServer::start_with_status(status.clone()).await;
    let (_, token) = server.register_and_issue_token().await;
    server.sign_up(Some(&token)).await.unwrap();

    let mut request = Request::new(SignInRequest {});
    request
        .metadata_mut()
        .insert("authorization", format!("Bearer {token}").parse().unwrap());
    let response = PlayerServiceClient::new(server.channel())
        .sign_in(request)
        .await
        .unwrap();
    let game_date = response
        .metadata()
        .get("x-game-date")
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(game_date, status.game_date().to_string());
    let server_time: i64 = response
        .metadata()
        .get("x-server-time")
        .unwrap()
        .to_str()
        .unwrap()
        .parse()
        .unwrap();
    let drift = (Timestamp::now().as_millisecond() - server_time).abs();
    assert!(drift < 60_000, "{server_time}");

    let yesterday = status.game_date().yesterday().unwrap().to_string();
    let mut request = Request::new(SignInRequest {});
    request
        .metadata_mut()
        .insert("authorization", format!("Bearer {token}").parse().unwrap());
    request
        .metadata_mut()
        .insert("x-game-date", yesterday.parse().unwrap());
    let status = PlayerServiceClient::new(server.channel())
        .sign_in(request)
        .await
        .unwrap_err();
    assert_reason(&status, reserved::DATE_CHANGED.reason());
}

/// 日付が違っても、トークンが無ければ先に UNAUTHENTICATED が返る(認証の後に日付を判定する)。
#[tokio::test]
async fn unauthenticated_takes_precedence_over_date_changed() {
    let server = TestServer::start_with_status(status_with(CONFIG)).await;

    let mut request = Request::new(SignInRequest {});
    request
        .metadata_mut()
        .insert("x-game-date", "2000-01-01".parse().unwrap());
    let status = PlayerServiceClient::new(server.channel())
        .sign_in(request)
        .await
        .unwrap_err();

    assert_eq!(status.code(), Code::Unauthenticated);
}
