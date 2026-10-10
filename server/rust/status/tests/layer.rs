use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use http::{HeaderMap, Request, Response};
use jiff::Timestamp;
use tonic::body::Body;
use tonic::{Code, Status};
use tonic_types::StatusExt;
use tower::{Layer, ServiceExt, service_fn};
use voltech_grpc_status::{
    BoxError, Clock, DateChangeLayer, ServerStatus, ServerStatusLayer, StatusConfig, StatusSource,
};

const CONFIG: &str = r#"
[maintenance]
enabled = false

[min_version]
ios = 120
android = 120
windows = 120

[game_day]
reset_hour = 4
timezone = "Asia/Tokyo"
"#;

// 2026-10-06 12:00 JST。ゲーム内の日付は 2026-10-06
const NOW: &str = "2026-10-06T03:00:00Z";

struct FixedClock(Timestamp);

impl Clock for FixedClock {
    fn now(&self) -> Timestamp {
        self.0
    }
}

fn status_with(config: &str) -> ServerStatus {
    ServerStatus::new(StatusConfig::from_toml(config).unwrap())
        .with_clock(FixedClock(NOW.parse().unwrap()))
}

fn request(headers: &[(&'static str, &str)]) -> Request<Body> {
    let mut request = Request::new(Body::empty());
    for (name, value) in headers {
        request.headers_mut().insert(*name, value.parse().unwrap());
    }
    request
}

// 中のサービスまで届いたら 200 と grpc-status 0 を返す
async fn ok_service(_: Request<Body>) -> Result<Response<Body>, Infallible> {
    let mut response = Response::new(Body::empty());
    response
        .headers_mut()
        .insert("grpc-status", "0".parse().unwrap());
    Ok(response)
}

async fn call_status_layer(status: ServerStatus, request: Request<Body>) -> Response<Body> {
    ServerStatusLayer::new(status)
        .layer(service_fn(ok_service))
        .oneshot(request)
        .await
        .unwrap()
}

async fn call_date_layer(status: ServerStatus, request: Request<Body>) -> Response<Body> {
    DateChangeLayer::new(status)
        .layer(service_fn(ok_service))
        .oneshot(request)
        .await
        .unwrap()
}

// レスポンスのヘッダーの grpc-status が OK でなければ、ErrorInfo の reason と metadata を返す
fn error_info(headers: &HeaderMap) -> Option<(String, std::collections::HashMap<String, String>)> {
    let status = Status::from_header_map(headers)?;
    if status.code() == Code::Ok {
        return None;
    }
    assert_eq!(status.code(), Code::FailedPrecondition);
    let info = status.get_details_error_info().unwrap();
    // 基盤のレイヤーが返すのは予約エラーだけ
    assert_eq!(info.domain, voltech_grpc_error::reserved::DOMAIN);
    Some((info.reason, info.metadata))
}

/// 最低バージョンより小さいバージョンには UPDATE_REQUIRED を返し、同じか大きいバージョン、エディタ、ヘッダーの無いリクエストは通す。
#[tokio::test]
async fn update_required_below_min_version() {
    let status = status_with(CONFIG);

    let response = call_status_layer(
        status.clone(),
        request(&[("x-app-platform", "ios"), ("x-app-version", "119")]),
    )
    .await;
    let (reason, _) = error_info(response.headers()).unwrap();
    assert_eq!(
        reason,
        voltech_grpc_error::reserved::UPDATE_REQUIRED.reason()
    );

    for headers in [
        vec![("x-app-platform", "ios"), ("x-app-version", "120")],
        vec![("x-app-platform", "android"), ("x-app-version", "121")],
        vec![("x-app-platform", "editor"), ("x-app-version", "1")],
        vec![],
        vec![("x-app-platform", "macos"), ("x-app-version", "1")],
    ] {
        let response = call_status_layer(status.clone(), request(&headers)).await;
        assert_eq!(error_info(response.headers()), None, "{headers:?}");
    }
}

/// メンテナンス中は全員に MAINTENANCE を返し、終了予定があれば metadata の end_at に UTC の RFC 3339 で入れる。
#[tokio::test]
async fn maintenance_returns_end_at() {
    let config = CONFIG.replace(
        "enabled = false",
        "enabled = true\nend_at = \"2026-10-06T15:00:00+09:00\"",
    );
    let response = call_status_layer(status_with(&config), request(&[])).await;

    let (reason, metadata) = error_info(response.headers()).unwrap();
    assert_eq!(reason, voltech_grpc_error::reserved::MAINTENANCE.reason());
    assert_eq!(metadata["end_at"], "2026-10-06T06:00:00Z");
}

/// 終了予定の無いメンテナンス(緊急メンテ)では、metadata に end_at を入れない。
#[tokio::test]
async fn maintenance_without_end_at() {
    let config = CONFIG.replace("enabled = false", "enabled = true");
    let response = call_status_layer(status_with(&config), request(&[])).await;

    let (reason, metadata) = error_info(response.headers()).unwrap();
    assert_eq!(reason, voltech_grpc_error::reserved::MAINTENANCE.reason());
    assert!(metadata.is_empty(), "{metadata:?}");
}

/// メンテナンスと強制アップデートが同時なら、強制アップデートを先に返す(メンテ中にアップデートを促せる)。
#[tokio::test]
async fn update_required_takes_precedence_over_maintenance() {
    let config = CONFIG.replace("enabled = false", "enabled = true");
    let response = call_status_layer(
        status_with(&config),
        request(&[("x-app-platform", "android"), ("x-app-version", "1")]),
    )
    .await;

    let (reason, _) = error_info(response.headers()).unwrap();
    assert_eq!(
        reason,
        voltech_grpc_error::reserved::UPDATE_REQUIRED.reason()
    );
}

/// 成功したレスポンスに、サーバー時刻(Unix ミリ秒)と今のゲーム内の日付を付ける。
#[tokio::test]
async fn response_has_server_time_and_game_date() {
    let response = call_status_layer(status_with(CONFIG), request(&[])).await;

    let now: Timestamp = NOW.parse().unwrap();
    assert_eq!(
        response.headers()["x-server-time"],
        now.as_millisecond().to_string()
    );
    assert_eq!(response.headers()["x-game-date"], "2026-10-06");
}

/// リクエストの日付が今のゲーム内の日付と違えば DATE_CHANGED を返し、同じか、ヘッダーが無ければ通す。
#[tokio::test]
async fn date_changed_when_game_date_differs() {
    let status = status_with(CONFIG);

    let response = call_date_layer(status.clone(), request(&[("x-game-date", "2026-10-05")])).await;
    let (reason, _) = error_info(response.headers()).unwrap();
    assert_eq!(reason, voltech_grpc_error::reserved::DATE_CHANGED.reason());

    for headers in [vec![("x-game-date", "2026-10-06")], vec![]] {
        let response = call_date_layer(status.clone(), request(&headers)).await;
        assert_eq!(error_info(response.headers()), None, "{headers:?}");
    }
}

// 読むたびに、差し替えた中身を返す読み込み元。None なら失敗する
#[derive(Clone)]
struct SwappableSource(Arc<Mutex<Option<String>>>);

impl StatusSource for SwappableSource {
    async fn load(&self) -> Result<StatusConfig, BoxError> {
        let text = self.0.lock().unwrap().clone().ok_or("unavailable")?;
        StatusConfig::from_toml(&text)
    }
}

async fn wait_until(mut condition: impl FnMut() -> bool) {
    for _ in 0..200 {
        if condition() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("timed out");
}

/// 設定は一定時間ごとに読み直される。読み直しに失敗したら、直前の設定を使い続ける。
#[tokio::test]
async fn watch_reloads_and_keeps_previous_on_failure() {
    let text = Arc::new(Mutex::new(Some(CONFIG.to_owned())));
    let status = ServerStatus::watch(SwappableSource(text.clone()), Duration::from_millis(10))
        .await
        .unwrap();
    assert!(!status.config().maintenance.enabled);

    *text.lock().unwrap() = Some(CONFIG.replace("enabled = false", "enabled = true"));
    wait_until(|| status.config().maintenance.enabled).await;

    *text.lock().unwrap() = None;
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(status.config().maintenance.enabled);
}

/// 最初の読み込みに失敗したら、エラーを返す(サーバーは起動しない)。
#[tokio::test]
async fn watch_fails_when_first_load_fails() {
    let source = SwappableSource(Arc::new(Mutex::new(None)));
    assert!(
        ServerStatus::watch(source, Duration::from_secs(1))
            .await
            .is_err()
    );
}

/// ハンドラーは、リクエストの extensions で今のゲーム内の日付(GameDate)を受け取れる。レスポンスヘッダーと同じ日付になる。
#[tokio::test]
async fn handler_receives_game_date() {
    let service = ServerStatusLayer::new(status_with(CONFIG)).layer(service_fn(
        |request: Request<Body>| async move {
            let date = request
                .extensions()
                .get::<voltech_grpc_status::GameDate>()
                .unwrap()
                .0;
            assert_eq!(date.to_string(), "2026-10-06");
            ok_service(request).await
        },
    ));

    let response = service.oneshot(request(&[])).await.unwrap();

    assert_eq!(response.headers()["x-game-date"], "2026-10-06");
}
