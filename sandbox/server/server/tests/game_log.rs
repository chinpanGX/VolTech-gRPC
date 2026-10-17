mod common;

use gacha::model::DRAW_COST;
use player::service::INITIAL_GEMS;
use serde_json::Value;
use ulid::Ulid;

use common::TestServer;

/// プレイヤーのゲームログ(method, kind, body)を、書いた順に返す。
async fn game_logs(server: &TestServer, player_id: Ulid) -> Vec<(String, String, Value)> {
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT method, kind, CAST(body AS CHAR) FROM game_logs WHERE player_id = ? ORDER BY log_id",
    )
    .bind(player_id.to_bytes().as_slice())
    .fetch_all(&server.pool)
    .await
    .unwrap();
    rows.into_iter()
        .map(|(method, kind, body)| (method, kind, serde_json::from_str(&body).unwrap()))
        .collect()
}

/// サインアップで初期データのジェムを、ガチャでジェムの消費とアイテムの付与を、ゲームログに残す。
/// 2回目のサインアップ(作成済み)と、再送で保存済みのレスポンスを返したガチャは残さない。
#[tokio::test]
async fn sign_up_and_draw_write_game_logs_once() {
    let server = TestServer::start().await;
    let (player_id, token) = server.register_and_issue_token().await;
    server.sign_up(Some(&token)).await.unwrap();
    server.sign_up(Some(&token)).await.unwrap();

    let request_id = Ulid::generate().to_string();
    let drawn = server
        .draw_with_request_id(&token, Some(&request_id))
        .await
        .unwrap();
    server
        .draw_with_request_id(&token, Some(&request_id))
        .await
        .unwrap();

    let logs = game_logs(&server, player_id).await;
    assert_eq!(logs.len(), 3, "{logs:?}");

    let (method, kind, body) = &logs[0];
    assert_eq!(method, "sandbox.v1.PlayerService/SignUp");
    assert_eq!(kind, "gems");
    assert_eq!(body["delta"], INITIAL_GEMS);
    assert_eq!(body["after"], INITIAL_GEMS);
    assert_eq!(body["reason"], "sign_up");

    let (method, kind, body) = &logs[1];
    assert_eq!(method, "sandbox.v1.GachaService/Draw");
    assert_eq!(kind, "gems");
    assert_eq!(body["delta"], -DRAW_COST);
    assert_eq!(body["after"], INITIAL_GEMS - DRAW_COST);
    assert_eq!(body["reason"], "gacha_draw");

    let item = drawn.item.unwrap();
    let (_, kind, body) = &logs[2];
    assert_eq!(kind, "item");
    assert_eq!(body["target_id"], item.item_id.to_string());
    assert_eq!(body["delta"], 1);
    assert_eq!(body["after"], item.count);
}

/// ジェムが足りずにガチャが失敗すると、ゲームログは残らない(サインアップの分だけ)。
#[tokio::test]
async fn failed_draw_writes_no_game_logs() {
    let server = TestServer::start().await;
    let (player_id, token) = server.register_and_issue_token().await;
    server.sign_up(Some(&token)).await.unwrap();
    server.set_gems(player_id, DRAW_COST - 1).await;

    server.draw(&token).await.unwrap_err();

    let logs = game_logs(&server, player_id).await;
    assert_eq!(logs.len(), 1, "{logs:?}");
    assert_eq!(logs[0].2["reason"], "sign_up");
}

/// 無料ガチャは、数量の変化でないゲームログ(free_draw)に、使ったゲーム内の日付を名前付きの値で残す。
#[tokio::test]
async fn free_draw_writes_event_game_log() {
    let server = TestServer::start().await;
    let (player_id, token) = server.register_and_issue_token().await;
    server.sign_up(Some(&token)).await.unwrap();

    server.draw_free(&token).await.unwrap();

    let logs = game_logs(&server, player_id).await;
    let (_, kind, body) = logs
        .iter()
        .find(|(_, kind, _)| kind == "free_draw")
        .unwrap();
    assert_eq!(kind, "free_draw");
    assert_eq!(body["reason"], "gacha_draw");
    let drawn_on = body["drawn_on"].as_str().unwrap();
    assert!(drawn_on.parse::<jiff::civil::Date>().is_ok(), "{drawn_on}");
    assert!(body.get("delta").is_none());
    assert!(body.get("after").is_none());
}
