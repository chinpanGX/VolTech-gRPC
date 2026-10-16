mod common;

use std::convert::Infallible;

use common::{Output, TestBody, config, subscriber};
use futures_util::FutureExt;
use http_body_util::BodyExt;
use serde_json::Value;
use tower::{Layer, Service, ServiceExt};
use voltech_grpc_error::{failed_precondition, reserved};
use voltech_grpc_log::{LogConfig, RpcLogLayer};

const DRAW_PATH: &str = "/app.v1.GachaService/Draw";

fn request(path: &str, request_id: Option<&str>) -> http::Request<()> {
    let mut builder = http::Request::builder().uri(path);
    if let Some(request_id) = request_id {
        builder = builder.header("x-request-id", request_id);
    }
    builder.body(()).unwrap()
}

/// `layer` を付けたサービスに `request` を送り、レスポンスの body を最後まで読んで、レスポンスのヘッダーを返す。
async fn call<S>(layer: RpcLogLayer, service: S, request: http::Request<()>) -> http::HeaderMap
where
    S: Service<http::Request<()>, Response = http::Response<TestBody>, Error = Infallible>
        + Clone
        + Send
        + 'static,
    S::Future: Send,
{
    let response = layer.layer(service).oneshot(request).await.unwrap();
    let headers = response.headers().clone();
    response.into_body().collect().await.unwrap();
    headers
}

fn only_access_log(output: &Output) -> Value {
    let logs = output.access_logs();
    assert_eq!(logs.len(), 1, "{logs:?}");
    logs.into_iter().next().unwrap()
}

/// 成功した RPC は、body の trailers のステータスで、RPC の名前・request_id・処理時間と一緒に info で1行出す。
#[tokio::test]
async fn ok_rpc_writes_one_access_log() {
    let output = Output::default();
    let _guard = tracing::subscriber::set_default(subscriber(&config(), &output));

    let service = tower::service_fn(|_| async { Ok(http::Response::new(TestBody::ok())) });
    call(
        RpcLogLayer::default(),
        service,
        request(DRAW_PATH, Some("req-1")),
    )
    .await;

    let log = only_access_log(&output);
    assert_eq!(log["level"], "INFO");
    assert_eq!(log["rpc.response.status_code"], "OK");
    assert_eq!(log["rpc.system.name"], "grpc");
    assert_eq!(log["rpc.method"], "app.v1.GachaService/Draw");
    assert_eq!(log["request_id"], "req-1");
    assert!(log["duration_ms"].is_f64());
    assert!(log.get("error.type").is_none());
    assert!(log.get("panic").is_none());
}

/// 業務エラー(Trailers-Only)は、ErrorInfo の domain と reason を付けて info で出し、error.type は付けない。
#[tokio::test]
async fn business_error_writes_reason_without_error_level() {
    let output = Output::default();
    let _guard = tracing::subscriber::set_default(subscriber(&config(), &output));

    let service = tower::service_fn(|_| async {
        Ok(failed_precondition(reserved::MAINTENANCE, "under maintenance").into_http())
    });
    call(RpcLogLayer::default(), service, request(DRAW_PATH, None)).await;

    let log = only_access_log(&output);
    assert_eq!(log["level"], "INFO");
    assert_eq!(log["rpc.response.status_code"], "FAILED_PRECONDITION");
    assert_eq!(log["error.domain"], reserved::MAINTENANCE.domain());
    assert_eq!(log["error.reason"], reserved::MAINTENANCE.reason());
    assert!(log.get("error.type").is_none());
}

/// ハンドラーの中で出したログにも、RPC の span の値が付く。
#[tokio::test]
async fn handler_logs_carry_rpc_fields() {
    let output = Output::default();
    let _guard = tracing::subscriber::set_default(subscriber(&config(), &output));

    let service = tower::service_fn(|_| async {
        tracing::warn!("inside handler");
        Ok(http::Response::new(TestBody::ok()))
    });
    call(
        RpcLogLayer::default(),
        service,
        request(DRAW_PATH, Some("req-1")),
    )
    .await;

    let line = output
        .lines()
        .into_iter()
        .find(|line| line["message"] == "inside handler")
        .unwrap();
    assert_eq!(line["request_id"], "req-1");
    assert_eq!(line["rpc.method"], "app.v1.GachaService/Draw");
}

/// 形が正しくない x-request-id は span に書かない。
#[tokio::test]
async fn invalid_request_id_is_not_recorded() {
    let output = Output::default();
    let _guard = tracing::subscriber::set_default(subscriber(&config(), &output));

    let service = tower::service_fn(|_| async { Ok(http::Response::new(TestBody::ok())) });
    let long = "a".repeat(65);
    for request_id in ["has space", "改行\n", long.as_str()] {
        let mut request = request(DRAW_PATH, None);
        if let Ok(value) = request_id.parse() {
            request.headers_mut().insert("x-request-id", value);
        }
        call(RpcLogLayer::default(), service, request).await;
    }

    let logs = output.access_logs();
    assert_eq!(logs.len(), 3);
    for log in logs {
        assert!(log.get("request_id").is_none(), "{log}");
    }
}

/// 既定では、ヘルスチェックとリフレクションのアクセスログを出さない。設定でメソッドまで含めた名前も除ける。
#[tokio::test]
async fn excluded_rpcs_write_no_access_log() {
    let output = Output::default();
    let _guard = tracing::subscriber::set_default(subscriber(&config(), &output));
    let service = tower::service_fn(|_| async { Ok(http::Response::new(TestBody::ok())) });

    for path in [
        "/grpc.health.v1.Health/Check",
        "/grpc.reflection.v1.ServerReflection/ServerReflectionInfo",
    ] {
        call(RpcLogLayer::default(), service, request(path, None)).await;
    }
    let mut config: LogConfig = config();
    config
        .excluded_rpcs
        .push("app.v1.GachaService/Draw".to_owned());
    call(RpcLogLayer::new(&config), service, request(DRAW_PATH, None)).await;

    assert!(output.access_logs().is_empty());
}

/// ハンドラーが panic すると、INTERNAL(INTERNAL_ERROR)を返し、アクセスログを error で出す。
#[tokio::test]
async fn panic_returns_internal_and_writes_error_log() {
    let output = Output::default();
    let _guard = tracing::subscriber::set_default(subscriber(&config(), &output));

    let service = tower::service_fn(|_: http::Request<()>| async {
        if true {
            panic!("boom");
        }
        Ok(http::Response::new(TestBody::ok()))
    });
    let headers = call(
        RpcLogLayer::default(),
        service,
        request(DRAW_PATH, Some("req-1")),
    )
    .await;

    assert_eq!(headers["grpc-status"], "13");
    let log = only_access_log(&output);
    assert_eq!(log["level"], "ERROR");
    assert_eq!(log["rpc.response.status_code"], "INTERNAL");
    assert_eq!(log["error.type"], "INTERNAL");
    assert_eq!(log["error.reason"], reserved::INTERNAL_ERROR.reason());
    assert_eq!(log["panic"], true);
    let internal = output
        .lines()
        .into_iter()
        .find(|line| line["message"] == "internal error")
        .unwrap();
    assert!(
        internal["exception.message"]
            .as_str()
            .unwrap()
            .contains("handler panicked: boom")
    );
    assert_eq!(internal["request_id"], "req-1");
}

/// レスポンスを返す前に future が捨てられる(クライアントの切断・期限切れ)と、CANCELLED のアクセスログを出す。
#[tokio::test]
async fn dropped_rpc_writes_cancelled_log() {
    let output = Output::default();
    let _guard = tracing::subscriber::set_default(subscriber(&config(), &output));

    let service = tower::service_fn(|_: http::Request<()>| async {
        std::future::pending::<()>().await;
        Ok::<_, Infallible>(http::Response::new(TestBody::ok()))
    });
    let future = RpcLogLayer::default()
        .layer(service)
        .oneshot(request(DRAW_PATH, None));
    assert!(future.now_or_never().is_none());

    let log = only_access_log(&output);
    assert_eq!(log["rpc.response.status_code"], "CANCELLED");
    assert_eq!(log["level"], "INFO");
}

/// authorization のメタデータ(トークン)はログに出さない。
#[tokio::test]
async fn authorization_is_never_logged() {
    let output = Output::default();
    let _guard = tracing::subscriber::set_default(subscriber(&config(), &output));

    let service = tower::service_fn(|_| async {
        tracing::info!("inside handler");
        Ok(http::Response::new(TestBody::ok()))
    });
    let mut request = request(DRAW_PATH, Some("req-1"));
    request
        .headers_mut()
        .insert("authorization", "Bearer secret-token-123".parse().unwrap());
    call(RpcLogLayer::default(), service, request).await;

    assert!(!output.text().contains("secret-token-123"));
}
