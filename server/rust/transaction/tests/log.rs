mod common;

use prost_types::Duration;
use tracing::Instrument;
use tracing::field::Empty;
use ulid::Ulid;
use voltech_grpc_log::RPC_SPAN_TARGET;
use voltech_grpc_transaction::Transactions;

use common::{BUY, DRAW, LOGGING, Output, request, start};

/// `RpcLogLayer` と同じ RPC の span の中で、冪等性の対象の RPC を `request_id` で呼び、span の `replayed` を返す。
async fn run_in_rpc_span(
    transactions: &Transactions,
    output: &Output,
    player_id: Ulid,
    request_id: &str,
) -> serde_json::Value {
    let span = tracing::info_span!(target: RPC_SPAN_TARGET, "rpc", replayed = Empty);
    transactions
        .run(&request(DRAW, Some(request_id)), Some(player_id), |_| {
            Box::pin(async { Ok(Duration::default()) })
        })
        .instrument(span.clone())
        .await
        .unwrap();
    span.in_scope(|| tracing::info!("finished"));
    output.lines().last().unwrap()["replayed"].clone()
}

/// 冪等性の対象の RPC は、処理を動かしたら false、再送で保存済みのレスポンスを返したら true を span の replayed に書く。
#[tokio::test]
async fn replayed_is_recorded_in_rpc_span() {
    let _serial = LOGGING.lock().await;
    let output = Output::default();
    let _guard = tracing::subscriber::set_default(common::subscriber(&output));
    let (transactions, _) = start().await;
    let player_id = Ulid::generate();

    assert_eq!(
        run_in_rpc_span(&transactions, &output, player_id, "req-1").await,
        false
    );
    assert_eq!(
        run_in_rpc_span(&transactions, &output, player_id, "req-1").await,
        true
    );
}

/// 同じ x-request-id を別の RPC で使い回すと、warn で、使った RPC の名前を出す。
#[tokio::test]
async fn reused_request_id_is_warned() {
    let _serial = LOGGING.lock().await;
    let output = Output::default();
    let _guard = tracing::subscriber::set_default(common::subscriber(&output));
    let (transactions, _) = start().await;
    let player_id = Ulid::generate();

    for method in [DRAW, BUY] {
        let _ = transactions
            .run(&request(method, Some("req-1")), Some(player_id), |_| {
                Box::pin(async { Ok(Duration::default()) })
            })
            .await;
    }

    let warns: Vec<_> = output
        .lines()
        .into_iter()
        .filter(|line| line["level"] == "WARN")
        .collect();
    assert_eq!(warns.len(), 1, "{warns:?}");
    assert_eq!(warns[0]["saved_method"], "test.v1.SampleService/Draw");
}
