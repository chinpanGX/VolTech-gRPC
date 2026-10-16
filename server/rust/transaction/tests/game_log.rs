mod common;

use prost_types::Duration;
use serde_json::Value;
use sqlx::MySqlPool;
use tonic::{Code, Status};
use ulid::Ulid;
use voltech_grpc_transaction::{GameLog, GameLogSink};

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
