mod common;

use player::service::INITIAL_GEMS;
use proto::errors;
use tonic::Code;
use tonic_types::StatusExt;
use ulid::Ulid;
use voltech_grpc_error::reserved;

use common::TestServer;

/// デバイス登録ではプレイヤー(players の行)は作られず、ID だけが発行される。
#[tokio::test]
async fn register_issues_player_id_without_creating_player() {
    let server = TestServer::start().await;
    let registered = server.register().await;
    let player_id = Ulid::from_string(&registered.player_id).unwrap();

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM players WHERE player_id = ?")
        .bind(player_id.to_bytes().as_slice())
        .fetch_one(&server.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

/// サインアップの前にサインインすると、FAILED_PRECONDITION + PLAYER_NOT_FOUND になる。
#[tokio::test]
async fn sign_in_before_sign_up_is_player_not_found() {
    let server = TestServer::start().await;
    let (_, token) = server.register_and_issue_token().await;

    let status = server.sign_in(Some(&token)).await.unwrap_err();
    assert_eq!(status.code(), Code::FailedPrecondition);
    let error_info = status.get_details_error_info().unwrap();
    assert_eq!(error_info.reason, errors::PLAYER_NOT_FOUND.reason());
}

/// サインアップすると、デバイス登録で発行された player_id と初期のジェムを持ったプレイヤーができ、その後のサインインで同じプレイヤーが返る。
#[tokio::test]
async fn signed_up_player_can_sign_in() {
    let server = TestServer::start().await;
    let (player_id, token) = server.register_and_issue_token().await;

    let created = server.sign_up(Some(&token)).await.unwrap();
    assert_eq!(created.gems, INITIAL_GEMS);
    assert_eq!(created.player_id, player_id.to_string());

    let signed_in = server.sign_in(Some(&token)).await.unwrap();
    assert_eq!(signed_in, created);
}

/// サインアップを2回呼んでもエラーにならず、2人目は作られずに同じプレイヤーが返る。
#[tokio::test]
async fn signing_up_twice_returns_same_player() {
    let server = TestServer::start().await;
    let (player_id, token) = server.register_and_issue_token().await;

    let first = server.sign_up(Some(&token)).await.unwrap();
    let second = server.sign_up(Some(&token)).await.unwrap();
    assert_eq!(second, first);

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM players WHERE player_id = ?")
        .bind(player_id.to_bytes().as_slice())
        .fetch_one(&server.pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

/// トークンを付けずにサインアップ・サインインすると UNAUTHENTICATED になる(AuthLayer が付いている)。
#[tokio::test]
async fn player_service_requires_token() {
    let server = TestServer::start().await;

    let status = server.sign_up(None).await.unwrap_err();
    assert_eq!(status.code(), Code::Unauthenticated);
    let status = server.sign_in(None).await.unwrap_err();
    assert_eq!(status.code(), Code::Unauthenticated);
}

/// 停止中のプレイヤーがサインインすると、FAILED_PRECONDITION + ACCOUNT_SUSPENDED になる
/// (アプリの PlayerLookup が AuthLayer に組み込まれていて、players テーブルの状態が認証に反映される)。
#[tokio::test]
async fn suspended_player_cannot_sign_in() {
    let server = TestServer::start().await;
    let (player_id, token) = server.register_and_issue_token().await;
    server.sign_up(Some(&token)).await.unwrap();
    sqlx::query("UPDATE players SET suspended_at = NOW(3) WHERE player_id = ?")
        .bind(player_id.to_bytes().as_slice())
        .execute(&server.pool)
        .await
        .unwrap();

    let status = server.sign_in(Some(&token)).await.unwrap_err();
    assert_eq!(status.code(), Code::FailedPrecondition);
    let error_info = status.get_details_error_info().unwrap();
    assert_eq!(error_info.reason, reserved::ACCOUNT_SUSPENDED.reason());
}
