mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use prost_types::Duration;
use sqlx::MySqlPool;
use tonic::{Code, Status};
use ulid::Ulid;

use common::{BUY, DRAW, READ, SIGN_IN, request, start};

fn response(seconds: i64) -> Duration {
    Duration { seconds, nanos: 0 }
}

/// 呼ばれた回数を数え、その回数をレスポンスにして返す処理を作る。
fn counting_handler(
    calls: &Arc<AtomicI64>,
) -> impl for<'t> FnOnce(
    &'t mut sqlx::Transaction<'static, sqlx::MySql>,
) -> voltech_grpc_transaction::HandlerFuture<'t, Duration>
+ Send
+ use<> {
    let calls = Arc::clone(calls);
    move |_| Box::pin(async move { Ok(response(calls.fetch_add(1, Ordering::SeqCst) + 1)) })
}

async fn last_response_exists(pool: &MySqlPool, player_id: Ulid) -> bool {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM last_responses WHERE player_id = ?")
        .bind(player_id.to_bytes().as_slice())
        .fetch_one(pool)
        .await
        .unwrap()
        == 1
}

/// 冪等性の対象の RPC を同じ RequestID で2回呼ぶと、2回目は本体を動かさずに、1回目のレスポンスが返る。
#[tokio::test]
async fn same_request_id_returns_saved_response() {
    let (transactions, _) = start().await;
    let player_id = Ulid::generate();
    let calls = Arc::new(AtomicI64::new(0));

    for _ in 0..2 {
        let result = transactions
            .run(
                &request(DRAW, Some("request-1")),
                Some(player_id),
                counting_handler(&calls),
            )
            .await
            .unwrap();
        assert_eq!(result, response(1));
    }
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

/// RequestID が変われば、本体をもう一度動かす。
#[tokio::test]
async fn new_request_id_runs_again() {
    let (transactions, _) = start().await;
    let player_id = Ulid::generate();
    let calls = Arc::new(AtomicI64::new(0));

    for request_id in ["request-1", "request-2"] {
        transactions
            .run(
                &request(DRAW, Some(request_id)),
                Some(player_id),
                counting_handler(&calls),
            )
            .await
            .unwrap();
    }
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

/// 同じ RequestID で別の RPC を呼ぶと、INVALID_ARGUMENT になり、本体は動かない。
#[tokio::test]
async fn same_request_id_for_another_method_is_invalid_argument() {
    let (transactions, _) = start().await;
    let player_id = Ulid::generate();
    let calls = Arc::new(AtomicI64::new(0));
    transactions
        .run(
            &request(DRAW, Some("request-1")),
            Some(player_id),
            counting_handler(&calls),
        )
        .await
        .unwrap();

    let status = transactions
        .run(
            &request(BUY, Some("request-1")),
            Some(player_id),
            counting_handler(&calls),
        )
        .await
        .unwrap_err();

    assert_eq!(status.code(), Code::InvalidArgument);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

/// 本体が失敗するとロールバックされ、レスポンスも保存されないので、同じ RequestID で送り直すと本体をもう一度動かす。
#[tokio::test]
async fn failure_is_rolled_back_and_not_saved() {
    let (transactions, pool) = start().await;
    let player_id = Ulid::generate();
    let written_in_failed_call = Ulid::generate();

    let status = transactions
        .run(
            &request(DRAW, Some("request-1")),
            Some(player_id),
            move |tx| {
                Box::pin(async move {
                    sqlx::query("INSERT INTO last_responses (player_id) VALUES (?)")
                        .bind(written_in_failed_call.to_bytes().to_vec())
                        .execute(&mut **tx)
                        .await
                        .unwrap();
                    Err::<Duration, _>(Status::failed_precondition("business error"))
                })
            },
        )
        .await
        .unwrap_err();
    assert_eq!(status.code(), Code::FailedPrecondition);
    assert!(!last_response_exists(&pool, written_in_failed_call).await);

    let calls = Arc::new(AtomicI64::new(0));
    transactions
        .run(
            &request(DRAW, Some("request-1")),
            Some(player_id),
            counting_handler(&calls),
        )
        .await
        .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

/// 冪等性の対象の RPC に x-request-id が無い・形が違うと、INVALID_ARGUMENT になり、本体は動かない。
#[tokio::test]
async fn target_without_valid_request_id_is_invalid_argument() {
    let (transactions, _) = start().await;
    let calls = Arc::new(AtomicI64::new(0));
    let long = "a".repeat(65);

    for request_id in [None, Some("has space"), Some(long.as_str())] {
        let status = transactions
            .run(
                &request(DRAW, request_id),
                Some(Ulid::generate()),
                counting_handler(&calls),
            )
            .await
            .unwrap_err();
        assert_eq!(status.code(), Code::InvalidArgument, "{request_id:?}");
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

/// 冪等性の対象の RPC をプレイヤー無しで呼ぶと、INTERNAL になる(ハンドラーの書き間違い)。
#[tokio::test]
async fn target_without_player_is_internal() {
    let (transactions, _) = start().await;
    let calls = Arc::new(AtomicI64::new(0));

    let status = transactions
        .run(
            &request(DRAW, Some("request-1")),
            None,
            counting_handler(&calls),
        )
        .await
        .unwrap_err();

    assert_eq!(status.code(), Code::Internal);
}

/// NO_SIDE_EFFECTS の RPC は読み取り専用のトランザクションで動き、書き込むとエラーになる。
#[tokio::test]
async fn no_side_effects_is_read_only() {
    let (transactions, _) = start().await;

    let write_failed = transactions
        .run(&request(READ, None), None, move |tx| {
            Box::pin(async move {
                let result = sqlx::query("INSERT INTO last_responses (player_id) VALUES (?)")
                    .bind(Ulid::generate().to_bytes().to_vec())
                    .execute(&mut **tx)
                    .await;
                Ok(response(i64::from(result.is_err())))
            })
        })
        .await
        .unwrap();

    assert_eq!(write_failed, response(1));
}

/// IDEMPOTENT の RPC はレスポンスを保存しないので、同じ RequestID でも毎回本体を動かす。
#[tokio::test]
async fn idempotent_runs_every_time() {
    let (transactions, _) = start().await;
    let player_id = Ulid::generate();
    let calls = Arc::new(AtomicI64::new(0));

    for _ in 0..2 {
        transactions
            .run(
                &request(SIGN_IN, Some("request-1")),
                Some(player_id),
                counting_handler(&calls),
            )
            .await
            .unwrap();
    }
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}
