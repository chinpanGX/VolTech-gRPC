mod common;

use std::time::Duration;

use tonic::Code;
use tonic_types::StatusExt;
use ulid::Ulid;
use voltech_grpc_auth::PlayerStatus;

use common::{SECRET_KEY, TestServer, start};

/// 登録したデバイスで発行したトークンを付けると、認証が必要なサービスを呼べる。
#[tokio::test]
async fn issued_token_passes_auth() {
    let server = start().await;
    let device_id = server.register().await;
    let token = server.issue_token(&device_id, SECRET_KEY).await.unwrap();

    server.call_protected(Some(&token)).await.unwrap();
}

/// デバイス登録で、device_id と一緒に ULID の player_id が発行される。登録するたびに別の player_id になる。
#[tokio::test]
async fn register_issues_player_id() {
    let server = start().await;
    let first = server.register_response(SECRET_KEY).await.unwrap();
    let second = server.register_response(SECRET_KEY).await.unwrap();

    assert!(
        Ulid::from_string(&first.player_id).is_ok(),
        "{}",
        first.player_id
    );
    assert_ne!(first.player_id, second.player_id);
}

/// トークンを付けずに認証が必要なサービスを呼ぶと UNAUTHENTICATED になる。
#[tokio::test]
async fn missing_token_is_unauthenticated() {
    let server = start().await;

    let status = server.call_protected(None).await.unwrap_err();
    assert_eq!(status.code(), Code::Unauthenticated);
}

/// 有効なトークンでも、`Bearer ` を付けない・大文字小文字が違う(`bearer`)と UNAUTHENTICATED になる。
#[tokio::test]
async fn authorization_without_exact_bearer_prefix_is_unauthenticated() {
    let server = start().await;
    let device_id = server.register().await;
    let token = server.issue_token(&device_id, SECRET_KEY).await.unwrap();

    for authorization in [
        token.clone(),
        format!("bearer {token}"),
        format!("BEARER {token}"),
    ] {
        let status = server
            .call_with_authorization(Some(&authorization))
            .await
            .unwrap_err();
        assert_eq!(
            status.code(),
            Code::Unauthenticated,
            "authorization = {authorization:?}"
        );
    }
    server.call_protected(Some(&token)).await.unwrap();
}

/// 発行していないトークンを付けると UNAUTHENTICATED になる。
#[tokio::test]
async fn unknown_token_is_unauthenticated() {
    let server = start().await;

    let status = server.call_protected(Some("unknown")).await.unwrap_err();
    assert_eq!(status.code(), Code::Unauthenticated);
}

/// 登録時と違う secret_key ではトークンを発行できず、UNAUTHENTICATED になる。
#[tokio::test]
async fn wrong_secret_key_is_unauthenticated() {
    let server = start().await;
    let device_id = server.register().await;

    let status = server
        .issue_token(&device_id, "wrong-secret-key-0000")
        .await
        .unwrap_err();
    assert_eq!(status.code(), Code::Unauthenticated);
}

/// 登録されていない device_id ではトークンを発行できず、UNAUTHENTICATED になる。
#[tokio::test]
async fn unknown_device_is_unauthenticated() {
    let server = start().await;

    let status = server
        .issue_token("01ARZ3NDEKTSV4RRFFQ69G5FAV", SECRET_KEY)
        .await
        .unwrap_err();
    assert_eq!(status.code(), Code::Unauthenticated);
}

/// secret_key が 16 バイトちょうど・128 バイトちょうどなら、登録してトークンを発行できる。
#[tokio::test]
async fn secret_key_at_length_limits_can_register() {
    let server = start().await;

    for secret_key in ["a".repeat(16), "a".repeat(128)] {
        let device_id = server.register_with(&secret_key).await.unwrap();
        server.issue_token(&device_id, &secret_key).await.unwrap();
    }
}

/// secret_key が空・15 バイト・129 バイトではデバイスを登録できず、INVALID_ARGUMENT になる。
#[tokio::test]
async fn secret_key_outside_length_limits_is_invalid_argument() {
    let server = start().await;

    for secret_key in [String::new(), "a".repeat(15), "a".repeat(129)] {
        let status = server.register_with(&secret_key).await.unwrap_err();
        assert_eq!(
            status.code(),
            Code::InvalidArgument,
            "len = {}",
            secret_key.len()
        );
    }
}

/// Guid.NewGuid().ToString() の形(ハイフン入り 36 文字)の secret_key で、登録してトークンを発行できる。
#[tokio::test]
async fn guid_secret_key_can_register() {
    let server = start().await;
    let secret_key = "3f2504e0-4f89-11d3-9a0c-0305e82c3301";

    let device_id = server.register_with(secret_key).await.unwrap();
    server.issue_token(&device_id, secret_key).await.unwrap();
}

/// 長さが範囲内でも、ASCII 以外・空白・制御文字を含む secret_key は INVALID_ARGUMENT になる。
#[tokio::test]
async fn non_printable_ascii_secret_key_is_invalid_argument() {
    let server = start().await;

    for secret_key in [
        "あいうえおかきくけこさしすせそた",
        "0123456789 abcdef",
        "0123456789\tabcdef",
        "0123456789abcdef\n",
    ] {
        let status = server.register_with(secret_key).await.unwrap_err();
        assert_eq!(
            status.code(),
            Code::InvalidArgument,
            "secret_key = {secret_key:?}"
        );
    }
}

/// IssueToken に長さが範囲外の secret_key を送っても、INVALID_ARGUMENT ではなく UNAUTHENTICATED になる。
#[tokio::test]
async fn issue_token_with_out_of_range_secret_key_is_unauthenticated() {
    let server = start().await;
    let device_id = server.register().await;

    for secret_key in [String::new(), "a".repeat(129)] {
        let status = server
            .issue_token(&device_id, &secret_key)
            .await
            .unwrap_err();
        assert_eq!(
            status.code(),
            Code::Unauthenticated,
            "len = {}",
            secret_key.len()
        );
    }
}

/// 同じ secret_key で2回登録すると別のデバイスになり、片方のトークンの発行がもう片方のトークンを無効にしない。
#[tokio::test]
async fn same_secret_key_registers_separate_devices() {
    let server = start().await;
    let first_device = server.register().await;
    let second_device = server.register().await;
    assert_ne!(first_device, second_device);

    let first_token = server.issue_token(&first_device, SECRET_KEY).await.unwrap();
    let second_token = server
        .issue_token(&second_device, SECRET_KEY)
        .await
        .unwrap();

    server.call_protected(Some(&first_token)).await.unwrap();
    server.call_protected(Some(&second_token)).await.unwrap();
}

/// トークンを発行し直すと古いトークンは使えなくなり、新しいトークンだけが使える。
#[tokio::test]
async fn reissuing_token_invalidates_old_one() {
    let server = start().await;
    let device_id = server.register().await;
    let old_token = server.issue_token(&device_id, SECRET_KEY).await.unwrap();
    let new_token = server.issue_token(&device_id, SECRET_KEY).await.unwrap();

    let status = server.call_protected(Some(&old_token)).await.unwrap_err();
    assert_eq!(status.code(), Code::Unauthenticated);
    server.call_protected(Some(&new_token)).await.unwrap();
}

/// 有効期限の切れたトークンを付けると UNAUTHENTICATED になる。
#[tokio::test]
async fn expired_token_is_unauthenticated() {
    let server = TestServer::start(PlayerStatus::Active, Duration::ZERO).await;
    let device_id = server.register().await;
    let token = server.issue_token(&device_id, SECRET_KEY).await.unwrap();

    let status = server.call_protected(Some(&token)).await.unwrap_err();
    assert_eq!(status.code(), Code::Unauthenticated);
}

/// プレイヤーがまだ作られていない(サインアップの前)デバイスでも、認証は通る。
#[tokio::test]
async fn player_not_created_passes_auth() {
    let server = TestServer::start(PlayerStatus::NotCreated, Duration::from_secs(3600)).await;
    let device_id = server.register().await;
    let token = server.issue_token(&device_id, SECRET_KEY).await.unwrap();

    server.call_protected(Some(&token)).await.unwrap();
}

/// 停止中のプレイヤーは、FAILED_PRECONDITION + ACCOUNT_SUSPENDED で弾かれる。
#[tokio::test]
async fn suspended_player_is_rejected_with_account_suspended() {
    let server = TestServer::start(PlayerStatus::Suspended, Duration::from_secs(3600)).await;
    let device_id = server.register().await;
    let token = server.issue_token(&device_id, SECRET_KEY).await.unwrap();

    let status = server.call_protected(Some(&token)).await.unwrap_err();
    assert_eq!(status.code(), Code::FailedPrecondition);
    let error_info = status.get_details_error_info().unwrap();
    assert_eq!(
        error_info.reason,
        voltech_grpc_error::code::ACCOUNT_SUSPENDED
    );
}
