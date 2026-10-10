use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use http::{HeaderMap, HeaderValue};
use tonic::Status;
use tonic::server::NamedService;
use tower::{Layer, Service};
use voltech_grpc_error::{failed_precondition, failed_precondition_with_metadata};

use crate::GameDate;
use crate::config::{Platform, StatusConfig};
use crate::status::ServerStatus;
use crate::{
    APP_PLATFORM_HEADER, APP_VERSION_HEADER, GAME_DATE_HEADER, MAINTENANCE_END_AT_KEY,
    SERVER_TIME_HEADER,
};

type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

/// 強制アップデートとメンテナンスを判定し、レスポンスにサーバー時刻とゲーム内の日付を付ける tower の Layer。
///
/// DB を使わないので、メンテナンスで DB を止めていても判定できる。認証より外側(全サービス)に付ける。
#[derive(Clone)]
pub struct ServerStatusLayer {
    status: ServerStatus,
}

impl ServerStatusLayer {
    pub fn new(status: ServerStatus) -> Self {
        Self { status }
    }
}

impl<S> Layer<S> for ServerStatusLayer {
    type Service = ServerStatusService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ServerStatusService {
            inner,
            status: self.status.clone(),
        }
    }
}

/// `ServerStatusLayer` を付けたサービス。
#[derive(Clone)]
pub struct ServerStatusService<S> {
    inner: S,
    status: ServerStatus,
}

impl<S, ReqBody, ResBody> Service<http::Request<ReqBody>> for ServerStatusService<S>
where
    S: Service<http::Request<ReqBody>, Response = http::Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send,
    ReqBody: Send + 'static,
    ResBody: Default,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut request: http::Request<ReqBody>) -> Self::Future {
        // poll_ready で準備ができたのは self.inner なので、そちらを持ち出して代わりに複製を残す
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
        let status = self.status.clone();

        Box::pin(async move {
            let config = status.config();
            // ハンドラーとレスポンスヘッダーで同じ日付を使うように、受け取ったときに1回だけ決める
            // (日付が変わる時刻をまたいだリクエストでも、処理に使った日付とクライアントに返す日付がずれない)
            let today = GameDate(config.game_day.date_at(status.now()));
            request.extensions_mut().insert(today);
            let mut response = match check(&config, request.headers()) {
                Ok(()) => inner.call(request).await?,
                Err(error) => error.into_http(),
            };

            let headers = response.headers_mut();
            headers.insert(
                SERVER_TIME_HEADER,
                HeaderValue::from(status.now().as_millisecond()),
            );
            if let Ok(date) = HeaderValue::from_str(&today.0.to_string()) {
                headers.insert(GAME_DATE_HEADER, date);
            }
            Ok(response)
        })
    }
}

// 強制アップデートを、メンテナンスより先に判定する。メンテナンスと同時に新しいバージョンを出すとき、
// 古いアプリのユーザーに、メンテナンス中のうちにアップデートを促せる
fn check(config: &StatusConfig, headers: &HeaderMap) -> Result<(), Status> {
    if let Some((platform, version)) = app_version(headers)
        && let Some(min_version) = config.min_version.get(platform)
        && version < min_version
    {
        return Err(failed_precondition(
            voltech_grpc_error::reserved::UPDATE_REQUIRED,
            "update required",
        ));
    }

    if config.maintenance.enabled {
        let mut metadata = HashMap::new();
        if let Some(end_at) = config.maintenance.end_at {
            metadata.insert(MAINTENANCE_END_AT_KEY.to_owned(), end_at.to_string());
        }
        return Err(failed_precondition_with_metadata(
            voltech_grpc_error::reserved::MAINTENANCE,
            "under maintenance",
            metadata,
        ));
    }

    Ok(())
}

// アプリのプラットフォームとバージョン。どちらかが無い・読めないときは判定しない(grpcurl などで呼ぶときのため)
fn app_version(headers: &HeaderMap) -> Option<(Platform, u32)> {
    let platform = headers
        .get(APP_PLATFORM_HEADER)?
        .to_str()
        .ok()?
        .parse()
        .ok()?;
    let version = headers
        .get(APP_VERSION_HEADER)?
        .to_str()
        .ok()?
        .parse()
        .ok()?;
    Some((platform, version))
}

/// リクエストのゲーム内の日付が今の日付と違えば `DATE_CHANGED` を返す tower の Layer。
///
/// 日付のヘッダーが無ければ判定しないので、トークンの発行とサインインは引っかからない。
/// 認証の内側(認証が必要なサービス)に付ける。
#[derive(Clone)]
pub struct DateChangeLayer {
    status: ServerStatus,
}

impl DateChangeLayer {
    pub fn new(status: ServerStatus) -> Self {
        Self { status }
    }
}

impl<S> Layer<S> for DateChangeLayer {
    type Service = DateChangeService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        DateChangeService {
            inner,
            status: self.status.clone(),
        }
    }
}

/// `DateChangeLayer` を付けたサービス。
#[derive(Clone)]
pub struct DateChangeService<S> {
    inner: S,
    status: ServerStatus,
}

impl<S: NamedService> NamedService for DateChangeService<S> {
    const NAME: &'static str = S::NAME;
}

impl<S, ReqBody, ResBody> Service<http::Request<ReqBody>> for DateChangeService<S>
where
    S: Service<http::Request<ReqBody>, Response = http::Response<ResBody>>,
    S::Future: Send + 'static,
    ResBody: Default + Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: http::Request<ReqBody>) -> Self::Future {
        let requested = request
            .headers()
            .get(GAME_DATE_HEADER)
            .map(|value| value.to_str().unwrap_or_default().to_owned());
        // ServerStatusLayer が決めた日付があれば、それと比べる(ハンドラーと同じ日付で判定する)
        let today = request
            .extensions()
            .get::<GameDate>()
            .map_or_else(|| self.status.game_date(), |date| date.0);
        if let Some(requested) = requested
            && requested != today.to_string()
        {
            let error = failed_precondition(
                voltech_grpc_error::reserved::DATE_CHANGED,
                "game date changed",
            );
            return Box::pin(async move { Ok(error.into_http()) });
        }

        Box::pin(self.inner.call(request))
    }
}
