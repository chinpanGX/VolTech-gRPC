mod common;

use prost_types::Duration;
use serde_json::Value;
use serde_json::json;
use sqlx::MySqlPool;
use tonic::{Code, Status};
use ulid::Ulid;
use voltech_grpc_transaction::{GameLog, GameLogSink, GameLogs};

use common::{
    DRAW, LOGGING, Output, READ, SIGN_IN, authenticated, request, start, start_with_sink,
};

/// 保存したゲームログ(request_id, method, kind, body)を、書いた順に返す。
async fn game_logs(
    pool: &MySqlPool,
    player_id: Ulid,
) -> Vec<(Option<String>, String, String, Value)> {
    let rows: Vec<(Option<String>, String, String, String)> = sqlx::query_as(
        "SELECT request_id, method, kind, CAST(body AS CHAR) FROM game_logs \
         WHERE player_id = ? ORDER BY log_id",
    )
    .bind(player_id.to_bytes().as_slice())
    .fetch_all(pool)
    .await
    .unwrap();
    rows.into_iter()
        .map(|(request_id, method, kind, body)| {
            (
                request_id,
                method,
                kind,
                serde_json::from_str(&body).unwrap(),
            )
        })
        .collect()
}

/// コミットすると、ゲームログが1件ずつ、認証したプレイヤー・x-request-id・RPC の名前と一緒に DB に残る。
#[tokio::test]
async fn game_logs_are_written_to_database_on_commit() {
    let (transactions, pool) = start().await;
    let player_id = Ulid::generate();

    transactions
        .run(
            &authenticated(request(DRAW, Some("req-1")), player_id),
            Some(player_id),
            |ctx| {
                Box::pin(async move {
                    ctx.write_game_log(GameLog::new("gems", -300, 2700, "gacha_draw"))?;
                    ctx.write_game_log(GameLog::new("item", 1, 3, "gacha_draw").target(1001))?;
                    Ok(Duration::default())
                })
            },
        )
        .await
        .unwrap();

    let logs = game_logs(&pool, player_id).await;
    assert_eq!(logs.len(), 2);
    let (request_id, method, kind, body) = &logs[0];
    assert_eq!(request_id.as_deref(), Some("req-1"));
    assert_eq!(method, "test.v1.SampleService/Draw");
    assert_eq!(kind, "gems");
    assert_eq!(body["delta"], -300);
    assert_eq!(body["after"], 2700);
    assert_eq!(body["reason"], "gacha_draw");
    assert!(body["target_id"].is_null());
    assert_eq!(logs[1].2, "item");
    assert_eq!(logs[1].3["target_id"], "1001");
}

/// 処理が失敗してロールバックすると、書いたゲームログも残らない。
#[tokio::test]
async fn rolled_back_game_logs_are_not_written() {
    let (transactions, pool) = start().await;
    let player_id = Ulid::generate();

    let status = transactions
        .run(
            &authenticated(request(SIGN_IN, None), player_id),
            Some(player_id),
            |ctx| {
                Box::pin(async move {
                    ctx.write_game_log(GameLog::new("gems", -300, 2700, "gacha_draw"))?;
                    Err::<Duration, _>(Status::failed_precondition("business error"))
                })
            },
        )
        .await
        .unwrap_err();

    assert_eq!(status.code(), Code::FailedPrecondition);
    assert!(game_logs(&pool, player_id).await.is_empty());
}

/// 再送で保存済みのレスポンスを返したときは処理を呼ばないので、ゲームログは最初の1回の分しか残らない。
#[tokio::test]
async fn replayed_request_writes_no_game_logs() {
    let (transactions, pool) = start().await;
    let player_id = Ulid::generate();

    for _ in 0..2 {
        transactions
            .run(
                &authenticated(request(DRAW, Some("req-1")), player_id),
                Some(player_id),
                |ctx| {
                    Box::pin(async move {
                        ctx.write_game_log(GameLog::new("gems", -300, 2700, "gacha_draw"))?;
                        Ok(Duration::default())
                    })
                },
            )
            .await
            .unwrap();
    }

    assert_eq!(game_logs(&pool, player_id).await.len(), 1);
}

/// NO_SIDE_EFFECTS の RPC でゲームログを書くと INTERNAL になる。
#[tokio::test]
async fn no_side_effects_cannot_write_game_logs() {
    let (transactions, pool) = start().await;
    let player_id = Ulid::generate();

    let status = transactions
        .run(
            &authenticated(request(READ, None), player_id),
            None,
            |ctx| {
                Box::pin(async move {
                    ctx.write_game_log(GameLog::new("gems", 0, 0, "read"))?;
                    Ok(Duration::default())
                })
            },
        )
        .await
        .unwrap_err();

    assert_eq!(status.code(), Code::Internal);
    assert!(game_logs(&pool, player_id).await.is_empty());
}

/// 認証していない(AuthContext の無い)リクエストでゲームログを書くと INTERNAL になる。
#[tokio::test]
async fn game_logs_need_auth_context() {
    let (transactions, _) = start().await;

    let status = transactions
        .run(&request(SIGN_IN, None), None, |ctx| {
            Box::pin(async move {
                ctx.write_game_log(GameLog::new("gems", 100, 100, "sign_up"))?;
                Ok(Duration::default())
            })
        })
        .await
        .unwrap_err();

    assert_eq!(status.code(), Code::Internal);
}

/// 出力先が標準出力なら、DB には書かず、コミットしたときだけ log_type が game の JSON を1件ずつ出す。
#[tokio::test]
async fn stdout_sink_writes_json_only_after_commit() {
    let _serial = LOGGING.lock().await;
    let output = Output::default();
    let _guard = tracing::subscriber::set_default(common::subscriber(&output));
    let (transactions, pool) = start_with_sink(GameLogSink::Stdout).await;
    let player_id = Ulid::generate();

    let failed = transactions
        .run(
            &authenticated(request(SIGN_IN, Some("req-1")), player_id),
            Some(player_id),
            |ctx| {
                Box::pin(async move {
                    ctx.write_game_log(GameLog::new("gems", -1, 0, "rolled_back"))?;
                    Err::<Duration, _>(Status::failed_precondition("business error"))
                })
            },
        )
        .await;
    assert!(failed.is_err());
    transactions
        .run(
            &authenticated(request(SIGN_IN, Some("req-2")), player_id),
            Some(player_id),
            |ctx| {
                Box::pin(async move {
                    ctx.write_game_log(GameLog::new("item", 1, 3, "login_bonus").target(1001))?;
                    Ok(Duration::default())
                })
            },
        )
        .await
        .unwrap();

    let lines: Vec<Value> = output
        .lines()
        .into_iter()
        .filter(|line| line["target"] == voltech_grpc_log::GAME_LOG_TARGET)
        .collect();
    assert_eq!(lines.len(), 1, "{lines:?}");
    let line = &lines[0];
    assert_eq!(line["log_type"], "game");
    assert_eq!(line["level"], "INFO");
    assert_eq!(line["player_id"], player_id.to_string());
    assert_eq!(line["request_id"], "req-2");
    assert_eq!(line["rpc.method"], "test.v1.SampleService/SignIn");
    assert_eq!(line["kind"], "item");
    assert_eq!(line["target_id"], "1001");
    assert_eq!(line["delta"], 1);
    assert_eq!(line["after"], 3);
    assert_eq!(line["reason"], "login_bonus");
    assert!(game_logs(&pool, player_id).await.is_empty());
}

/// 業務データの代わりに、`player_id` の直前のレスポンスの行が保存されているかを返す。
async fn last_response_exists(pool: &MySqlPool, player_id: Ulid) -> bool {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM last_responses WHERE player_id = ?")
        .bind(player_id.to_bytes().as_slice())
        .fetch_one(pool)
        .await
        .unwrap()
        == 1
}

/// 数量の変化でないゲームログは、body に delta・after を入れず、名前付きの値を入れる。
#[tokio::test]
async fn event_game_log_stores_named_fields_without_quantity() {
    let (transactions, pool) = start().await;
    let player_id = Ulid::generate();

    transactions
        .run(
            &authenticated(request(SIGN_IN, None), player_id),
            Some(player_id),
            |ctx| {
                Box::pin(async move {
                    ctx.write_game_log(
                        GameLog::event("party", "edit_party")
                            .field("party_id", 1)
                            .field("unit_ids", json!(["u-1", "u-2", null])),
                    )?;
                    Ok(Duration::default())
                })
            },
        )
        .await
        .unwrap();

    let logs = game_logs(&pool, player_id).await;
    assert_eq!(logs.len(), 1);
    let (request_id, _, kind, body) = &logs[0];
    assert_eq!(*request_id, None);
    assert_eq!(kind, "party");
    assert_eq!(
        *body,
        json!({"reason": "edit_party", "party_id": 1, "unit_ids": ["u-1", "u-2", null]})
    );
}

/// `Transactions::run` を通らない処理では、`GameLogs` に複数のプレイヤーの分を書き、
/// `GameLogSink::commit` で業務データと同じトランザクションに書いてコミットする。
#[tokio::test]
async fn game_logs_outside_run_are_committed_with_transaction() {
    let (_, pool) = start().await;
    let winner = Ulid::generate();
    let loser = Ulid::generate();

    let mut tx = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO last_responses (player_id) VALUES (?)")
        .bind(winner.to_bytes().as_slice())
        .execute(&mut *tx)
        .await
        .unwrap();
    let mut logs =
        GameLogs::new("internal.v1.MatchService/ReportResult").with_request_id("match-1");
    logs.write(winner, GameLog::new("gems", 100, 3100, "match_win"))
        .unwrap();
    logs.write(
        loser,
        GameLog::event("match", "match_result")
            .field("match_id", "m-1")
            .field("result", "lose"),
    )
    .unwrap();
    GameLogSink::Database.commit(tx, logs).await.unwrap();

    assert!(last_response_exists(&pool, winner).await);
    let winner_logs = game_logs(&pool, winner).await;
    assert_eq!(winner_logs.len(), 1);
    assert_eq!(winner_logs[0].0.as_deref(), Some("match-1"));
    assert_eq!(winner_logs[0].1, "internal.v1.MatchService/ReportResult");
    assert_eq!(winner_logs[0].3["delta"], 100);
    let loser_logs = game_logs(&pool, loser).await;
    assert_eq!(loser_logs.len(), 1);
    assert_eq!(
        loser_logs[0].3,
        json!({"reason": "match_result", "match_id": "m-1", "result": "lose"})
    );
}

/// `GameLogSink::commit` を呼ばずにトランザクションをロールバックすると、ゲームログも業務データも残らない。
#[tokio::test]
async fn game_logs_outside_run_are_not_written_without_commit() {
    let (_, pool) = start().await;
    let player_id = Ulid::generate();

    let mut tx = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO last_responses (player_id) VALUES (?)")
        .bind(player_id.to_bytes().as_slice())
        .execute(&mut *tx)
        .await
        .unwrap();
    let mut logs = GameLogs::new("job/SeasonReward");
    logs.write(player_id, GameLog::new("gems", 100, 100, "season_reward"))
        .unwrap();
    tx.rollback().await.unwrap();
    drop(logs);

    assert!(!last_response_exists(&pool, player_id).await);
    assert!(game_logs(&pool, player_id).await.is_empty());
}

/// 処理の名前が使えない形なら、`commit` はエラーを返し、業務データもコミットしない。
#[tokio::test]
async fn invalid_method_fails_commit_and_rolls_back() {
    let (_, pool) = start().await;
    let player_id = Ulid::generate();

    for method in ["", "日本語の名前"] {
        let mut tx = pool.begin().await.unwrap();
        sqlx::query("INSERT INTO last_responses (player_id) VALUES (?)")
            .bind(player_id.to_bytes().as_slice())
            .execute(&mut *tx)
            .await
            .unwrap();
        let mut logs = GameLogs::new(method);
        logs.write(player_id, GameLog::new("gems", 1, 1, "test"))
            .unwrap();

        assert!(
            GameLogSink::Database.commit(tx, logs).await.is_err(),
            "{method:?}"
        );
        assert!(!last_response_exists(&pool, player_id).await);
    }
}

/// 値の名前が使えない形・基盤が付ける項目と同じ・重なっているときは、書くときにエラーになる。
#[test]
fn invalid_field_names_are_rejected() {
    let player_id = Ulid::generate();
    let mut logs = GameLogs::new("job/Test");
    for log in [
        GameLog::event("unit", "test").field("", 1),
        GameLog::event("unit", "test").field("Unit_ID", 1),
        GameLog::event("unit", "test").field("a".repeat(65), 1),
        GameLog::event("unit", "test").field("player_id", 1),
        GameLog::event("unit", "test").field("delta", 1),
        GameLog::event("unit", "test").field("severity", 1),
        GameLog::event("unit", "test")
            .field("unit_id", 1)
            .field("unit_id", 2),
    ] {
        assert!(logs.write(player_id, log.clone()).is_err(), "{log:?}");
    }
    assert!(logs.is_empty());
    logs.write(
        player_id,
        GameLog::event("unit", "test").field("unit.slot_2", 1),
    )
    .unwrap();
}

/// 形の正しくない x-request-id は付けない。
#[tokio::test]
async fn invalid_request_id_is_not_written() {
    let (_, pool) = start().await;
    let player_id = Ulid::generate();

    let tx = pool.begin().await.unwrap();
    let mut logs = GameLogs::new("job/Test").with_request_id("has space");
    logs.write(player_id, GameLog::new("gems", 1, 1, "test"))
        .unwrap();
    GameLogSink::Database.commit(tx, logs).await.unwrap();

    assert_eq!(game_logs(&pool, player_id).await[0].0, None);
}

/// 標準出力では、`run` の外のゲームログもコミットの後に出し、名前付きの値は一番上の項目に、
/// 数量の無いものは delta・after を付けずに出す。
#[tokio::test]
async fn stdout_game_logs_outside_run_flatten_named_fields() {
    let _serial = LOGGING.lock().await;
    let output = Output::default();
    let _guard = tracing::subscriber::set_default(common::subscriber(&output));
    let (_, pool) = start().await;
    let player_id = Ulid::generate();

    let tx = pool.begin().await.unwrap();
    let mut logs = GameLogs::new("internal.v1.UnitService/Grant");
    logs.write(
        player_id,
        GameLog::event("unit", "gacha_draw")
            .target("u-1")
            .field("unit_type_id", 3001),
    )
    .unwrap();
    GameLogSink::Stdout.commit(tx, logs).await.unwrap();

    let lines: Vec<Value> = output
        .lines()
        .into_iter()
        .filter(|line| line["target"] == voltech_grpc_log::GAME_LOG_TARGET)
        .collect();
    assert_eq!(lines.len(), 1, "{lines:?}");
    let line = &lines[0];
    assert_eq!(line["log_type"], "game");
    assert_eq!(line["player_id"], player_id.to_string());
    assert_eq!(line["rpc.method"], "internal.v1.UnitService/Grant");
    assert_eq!(line["kind"], "unit");
    assert_eq!(line["target_id"], "u-1");
    assert_eq!(line["unit_type_id"], 3001);
    assert!(line.get("delta").is_none());
    assert!(line.get("after").is_none());
    assert!(line.get("request_id").is_none());
    assert!(line.get(voltech_grpc_log::GAME_LOG_FIELDS).is_none());
    assert!(game_logs(&pool, player_id).await.is_empty());
}
