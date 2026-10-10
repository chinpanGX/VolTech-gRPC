mod common;

use gacha::model::{DRAW_COST, ITEMS};
use player::service::INITIAL_GEMS;
use proto::errors;
use proto::sandbox::v1::notification::Event;
use proto::sandbox::v1::{Badge, BadgeType, RareItemObtained};
use tonic::Code;
use tonic_types::StatusExt;

use common::TestServer;

fn free_gacha_badge(visible: bool) -> Badge {
    Badge {
        r#type: BadgeType::FreeGacha.into(),
        visible,
        count: 0,
    }
}

/// カウンターが 0 のときのカウンターのバッジ(非表示)。
fn hidden_counter_badge() -> Badge {
    Badge {
        r#type: BadgeType::Counter.into(),
        visible: false,
        count: 0,
    }
}

/// サインインでは全種類のバッジが返る。まだ無料ガチャを引いていなければ、無料ガチャのバッジは表示。カウンターが 0 ならカウンターのバッジは非表示。
#[tokio::test]
async fn sign_in_returns_all_badges() {
    let server = TestServer::start().await;
    let (_, token) = server.register_and_issue_token().await;
    server.sign_up(Some(&token)).await.unwrap();

    let signed_in = server.sign_in_response(&token).await;

    assert_eq!(
        signed_in.badges,
        vec![free_gacha_badge(true), hidden_counter_badge()]
    );
}

/// 無料ガチャはジェムを使わずに引け、レスポンスで無料ガチャのバッジが非表示になる。
/// 同じ日の2回目は FREE_DRAW_USED で、何も変わらない。サインインのバッジも非表示のまま。
#[tokio::test]
async fn free_draw_once_per_day_and_hides_badge() {
    let server = TestServer::start().await;
    let (_, token) = server.register_and_issue_token().await;
    server.sign_up(Some(&token)).await.unwrap();

    let drawn = server.draw_free(&token).await.unwrap();
    let diff = drawn.player_diff.unwrap();
    assert_eq!(diff.gems, None, "無料ガチャはジェムを使わない");
    assert_eq!(diff.items.unwrap().upserted.len(), 1);
    assert_eq!(drawn.badges, vec![free_gacha_badge(false)]);

    let status = server.draw_free(&token).await.unwrap_err();
    assert_eq!(status.code(), Code::FailedPrecondition);
    assert_eq!(
        status.get_details_error_info().unwrap().reason,
        errors::FREE_DRAW_USED.reason()
    );

    let signed_in = server.sign_in_response(&token).await;
    assert_eq!(signed_in.player.unwrap().gems, INITIAL_GEMS);
    assert_eq!(
        signed_in.items.iter().map(|item| item.count).sum::<i64>(),
        1
    );
    assert_eq!(
        signed_in.badges,
        vec![free_gacha_badge(false), hidden_counter_badge()]
    );
}

/// ジェムで引いても、無料ガチャのバッジは返らない(状態が変わらないので、記録されない)。
#[tokio::test]
async fn paid_draw_returns_no_badges() {
    let server = TestServer::start().await;
    let (_, token) = server.register_and_issue_token().await;
    server.sign_up(Some(&token)).await.unwrap();

    let drawn = server.draw(&token).await.unwrap();

    assert!(drawn.badges.is_empty(), "{:?}", drawn.badges);
}

/// レアのアイテムが出たときだけ、RareItemObtained の通知が返る。レアでなければ通知は無い。
///
/// 排出はランダムなので、レアが出るまで(最大 200 回)引く。レアの確率は 5% なので、200 回で出ない確率は 0.004% ほど。
#[tokio::test]
async fn rare_item_returns_notification() {
    let server = TestServer::start().await;
    let (player_id, token) = server.register_and_issue_token().await;
    server.sign_up(Some(&token)).await.unwrap();
    server.set_gems(player_id, DRAW_COST * 200).await;

    let mut rare_seen = false;
    for _ in 0..200 {
        let drawn = server.draw(&token).await.unwrap();
        let item_id = drawn.item.unwrap().item_id;
        let rare = ITEMS
            .iter()
            .any(|item| item.item_id == item_id && item.rare);
        let events: Vec<_> = drawn
            .notifications
            .into_iter()
            .map(|notification| notification.event.unwrap())
            .collect();
        if rare {
            assert_eq!(
                events,
                vec![Event::RareItemObtained(RareItemObtained { item_id })]
            );
            rare_seen = true;
            break;
        }
        assert!(events.is_empty(), "item {item_id}: {events:?}");
    }
    assert!(rare_seen, "200 回引いてもレアが出なかった");
}
