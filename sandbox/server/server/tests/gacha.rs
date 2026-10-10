mod common;

use gacha::model::{DRAW_COST, ITEMS};
use player::service::INITIAL_GEMS;
use proto::errors;
use tonic::Code;
use tonic_types::StatusExt;

use common::TestServer;

/// ガチャを引くと、ジェムが1回分減り、表にあるアイテムが1つ付与され、サインインの全件にも載る。
#[tokio::test]
async fn draw_consumes_gems_and_grants_one_item() {
    let server = TestServer::start().await;
    let (_, token) = server.register_and_issue_token().await;
    server.sign_up(Some(&token)).await.unwrap();

    let drawn = server.draw(&token).await.unwrap();

    assert_eq!(
        drawn.player_diff.unwrap().gems,
        Some(INITIAL_GEMS - DRAW_COST)
    );
    let item = drawn.item.unwrap();
    assert!(
        ITEMS
            .iter()
            .any(|candidate| candidate.item_id == item.item_id)
    );
    assert_eq!(item.count, 1);

    let signed_in = server.sign_in_response(&token).await;
    assert_eq!(signed_in.player.unwrap().gems, INITIAL_GEMS - DRAW_COST);
    assert_eq!(signed_in.items, vec![item]);
}

/// 同じアイテムが出たら個数が増え、引いた回数と所持数の合計が一致する。
#[tokio::test]
async fn drawing_repeatedly_accumulates_items() {
    let server = TestServer::start().await;
    let (_, token) = server.register_and_issue_token().await;
    server.sign_up(Some(&token)).await.unwrap();

    for _ in 0..5 {
        server.draw(&token).await.unwrap();
    }

    let signed_in = server.sign_in_response(&token).await;
    assert_eq!(signed_in.player.unwrap().gems, INITIAL_GEMS - DRAW_COST * 5);
    let total: i64 = signed_in.items.iter().map(|item| item.count).sum();
    assert_eq!(total, 5);
}

/// ジェムが1回分に足りないと、FAILED_PRECONDITION + INSUFFICIENT_GEMS になり、ジェムもアイテムも変わらない。
#[tokio::test]
async fn draw_without_enough_gems_changes_nothing() {
    let server = TestServer::start().await;
    let (player_id, token) = server.register_and_issue_token().await;
    server.sign_up(Some(&token)).await.unwrap();
    server.set_gems(player_id, DRAW_COST - 1).await;

    let status = server.draw(&token).await.unwrap_err();

    assert_eq!(status.code(), Code::FailedPrecondition);
    let error_info = status.get_details_error_info().unwrap();
    assert_eq!(error_info.reason, errors::INSUFFICIENT_GEMS.reason());
    let signed_in = server.sign_in_response(&token).await;
    assert_eq!(signed_in.player.unwrap().gems, DRAW_COST - 1);
    assert!(signed_in.items.is_empty());
}

/// サインアップの前にガチャを引くと、PLAYER_NOT_FOUND になる。
#[tokio::test]
async fn draw_before_sign_up_is_player_not_found() {
    let server = TestServer::start().await;
    let (_, token) = server.register_and_issue_token().await;

    let status = server.draw(&token).await.unwrap_err();

    assert_eq!(status.code(), Code::FailedPrecondition);
    let error_info = status.get_details_error_info().unwrap();
    assert_eq!(error_info.reason, errors::PLAYER_NOT_FOUND.reason());
}

/// 同じ RequestID で2回送ると、2回目は保存済みのレスポンスが返り、ジェムは1回分しか減らない。
#[tokio::test]
async fn resending_same_request_id_draws_once() {
    let server = TestServer::start().await;
    let (_, token) = server.register_and_issue_token().await;
    server.sign_up(Some(&token)).await.unwrap();

    let first = server
        .draw_with_request_id(&token, Some("request-1"))
        .await
        .unwrap();
    let resent = server
        .draw_with_request_id(&token, Some("request-1"))
        .await
        .unwrap();

    assert_eq!(resent, first);
    let signed_in = server.sign_in_response(&token).await;
    assert_eq!(signed_in.player.unwrap().gems, INITIAL_GEMS - DRAW_COST);
    let total: i64 = signed_in.items.iter().map(|item| item.count).sum();
    assert_eq!(total, 1);
}

/// x-request-id を付けずにガチャを引くと、INVALID_ARGUMENT になり、ジェムは減らない。
#[tokio::test]
async fn draw_without_request_id_is_invalid_argument() {
    let server = TestServer::start().await;
    let (_, token) = server.register_and_issue_token().await;
    server.sign_up(Some(&token)).await.unwrap();

    let status = server.draw_with_request_id(&token, None).await.unwrap_err();

    assert_eq!(status.code(), Code::InvalidArgument);
    let signed_in = server.sign_in_response(&token).await;
    assert_eq!(signed_in.player.unwrap().gems, INITIAL_GEMS);
}

/// ガチャのレスポンスの player_diff には、変更後のジェムと、出たアイテムの変更後の個数だけが入る。
#[tokio::test]
async fn draw_returns_player_diff_with_new_values() {
    let server = TestServer::start().await;
    let (_, token) = server.register_and_issue_token().await;
    server.sign_up(Some(&token)).await.unwrap();

    let first = server.draw(&token).await.unwrap();
    let second = server.draw(&token).await.unwrap();

    let diff = second.player_diff.unwrap();
    assert_eq!(diff.gems, Some(INITIAL_GEMS - DRAW_COST * 2));
    let items = diff.items.unwrap();
    assert!(items.removed.is_empty());
    let drawn = second.item.unwrap();
    assert_eq!(items.upserted, vec![drawn]);
    // 2回とも同じアイテムなら個数は2、違えば1(差分は変化量ではなく変更後の値)
    let expected_count = if first.item.unwrap().item_id == drawn.item_id {
        2
    } else {
        1
    };
    assert_eq!(drawn.count, expected_count);
}
