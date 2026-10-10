//! SandboxService(基盤の機能を1つずつ試す RPC)で、認証・エラー・冪等性・共通レスポンス項目が使えることを確かめる。

mod common;

use proto::errors;
use proto::sandbox::v1::notification::Event;
use proto::sandbox::v1::{
    BadgeType, FailRequest, IncrementRequest, IncrementResponse, PingReceived, PingRequest,
};
use tonic::Code;
use tonic_types::StatusExt;
use ulid::Ulid;
use voltech_grpc_error::{ErrorCode, reserved};

use common::{TestServer, request};

async fn increment(
    server: &TestServer,
    token: &str,
    request_id: &str,
    amount: i64,
) -> IncrementResponse {
    server
        .sandbox()
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
    let server = TestServer::start().await;
    let (player_id, token) = server.register_and_issue_token().await;

    let status = server
        .sandbox()
        .ping(request(PingRequest {}, &token, None))
        .await
        .unwrap_err();
    assert_eq!(status.code(), Code::FailedPrecondition);
    let info = status.get_details_error_info().unwrap();
    assert_eq!(info.domain, errors::DOMAIN);
    assert_eq!(info.reason, errors::PLAYER_NOT_FOUND.reason());

    server.sign_up(Some(&token)).await.unwrap();
    let response = server
        .sandbox()
        .ping(request(PingRequest {}, &token, None))
        .await
        .unwrap();
    assert!(response.metadata().get("x-server-time").is_some());
    assert!(response.metadata().get("x-game-date").is_some());
    let ping = response.into_inner();
    assert_eq!(ping.player_id, player_id.to_string());
    assert_eq!(ping.player_diff.unwrap().counter, Some(0));
    assert_eq!(ping.badges.len(), 1);
    assert_eq!(ping.badges[0].r#type(), BadgeType::Counter);
    assert!(!ping.badges[0].visible);
    let events: Vec<_> = ping
        .notifications
        .into_iter()
        .map(|notification| notification.event.unwrap())
        .collect();
    assert_eq!(events, vec![Event::PingReceived(PingReceived {})]);
}

/// Fail は指定したエラーコードの FAILED_PRECONDITION で失敗する。指定が無ければ INTERNAL_ERROR、知らないコードなら INVALID_ARGUMENT。
#[tokio::test]
async fn fail_returns_requested_error_code() {
    let server = TestServer::start().await;
    let (_, token) = server.register_and_issue_token().await;

    for (requested, expected) in [
        (errors::SANDBOX_STAY, errors::SANDBOX_STAY),
        (errors::INSUFFICIENT_GEMS, errors::INSUFFICIENT_GEMS),
        (reserved::MAINTENANCE, reserved::MAINTENANCE),
        (ErrorCode::new("", ""), reserved::INTERNAL_ERROR),
    ] {
        let status = server
            .sandbox()
            .fail(request(fail_request(requested), &token, None))
            .await
            .unwrap_err();
        assert_eq!(status.code(), Code::FailedPrecondition);
        let info = status.get_details_error_info().unwrap();
        assert_eq!(info.domain, expected.domain());
        assert_eq!(info.reason, expected.reason());
    }

    // 基盤の予約エラーの reason でも、domain が違えば別のエラー
    let status = server
        .sandbox()
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
/// 足した後のカウンターは player_diff にも入り、サインインでも読める。
#[tokio::test]
async fn increment_with_same_request_id_is_applied_once() {
    let server = TestServer::start().await;
    let (_, token) = server.register_and_issue_token().await;
    server.sign_up(Some(&token)).await.unwrap();

    let request_id = Ulid::generate().to_string();
    let first = increment(&server, &token, &request_id, 5).await;
    let resent = increment(&server, &token, &request_id, 5).await;
    assert_eq!(first.counter, 5);
    assert_eq!(first.player_diff.as_ref().unwrap().counter, Some(5));
    assert_eq!(resent, first);

    let next = increment(&server, &token, &Ulid::generate().to_string(), 5).await;
    assert_eq!(next.counter, 10);
    assert_eq!(server.sign_in(Some(&token)).await.unwrap().counter, 10);
}

/// 冪等性の対象の Increment は、x-request-id が無いと失敗する。
#[tokio::test]
async fn increment_without_request_id_is_rejected() {
    let server = TestServer::start().await;
    let (_, token) = server.register_and_issue_token().await;
    server.sign_up(Some(&token)).await.unwrap();

    let status = server
        .sandbox()
        .increment(request(IncrementRequest { amount: 1 }, &token, None))
        .await
        .unwrap_err();
    assert_eq!(status.code(), Code::InvalidArgument);
}
