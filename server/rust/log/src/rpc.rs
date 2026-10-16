use std::any::Any;
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, ready};
use std::time::Instant;

use futures_util::FutureExt;
use http::HeaderMap;
use http_body::{Body, Frame, SizeHint};
use pin_project_lite::pin_project;
use tonic::{Code, Status};
use tonic_types::StatusExt;
use tower::{Layer, Service};
use tracing::field::Empty;
use tracing::{Instrument, Level, Span};
use voltech_grpc_error::Internal;

use crate::config::{DEFAULT_EXCLUDED_RPCS, LogConfig};
use crate::{
    ACCESS_LOG_TARGET, REQUEST_ID_HEADER, RPC_SPAN_TARGET, RpcMethod, field, is_valid_request_id,
};

type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

/// RPC ごとに span を作り、RPC の終わりにアクセスログを1行出す tower の Layer。
///
/// span の中で出たログには `request_id`・`rpc.method` などが付く。全サービスの一番外側に付ける。
/// ハンドラーが panic したときは、アクセスログを出してから INTERNAL を返す。
#[derive(Clone, Debug)]
pub struct RpcLogLayer {
    excluded_rpcs: Arc<[String]>,
}

impl RpcLogLayer {
    /// 設定のうち、アクセスログから除く RPC を使って作る。
    pub fn new(config: &LogConfig) -> Self {
        Self {
            excluded_rpcs: config.excluded_rpcs.iter().cloned().collect(),
        }
    }
}

impl Default for RpcLogLayer {
    /// アクセスログから除く RPC を既定(`DEFAULT_EXCLUDED_RPCS`)にして作る。
    fn default() -> Self {
        Self {
            excluded_rpcs: DEFAULT_EXCLUDED_RPCS
                .iter()
                .map(|&rpc| rpc.to_owned())
                .collect(),
        }
    }
}

impl<S> Layer<S> for RpcLogLayer {
    type Service = RpcLogService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RpcLogService {
            inner,
            excluded_rpcs: self.excluded_rpcs.clone(),
        }
    }
}

/// `RpcLogLayer` を付けたサービス。
#[derive(Clone, Debug)]
pub struct RpcLogService<S> {
    inner: S,
    excluded_rpcs: Arc<[String]>,
}

impl<S, ReqBody, ResBody> Service<http::Request<ReqBody>> for RpcLogService<S>
where
    S: Service<http::Request<ReqBody>, Response = http::Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send,
    ReqBody: Send + 'static,
    ResBody: Default + Send + 'static,
{
    type Response = http::Response<AccessLogBody<ResBody>>;
    type Error = S::Error;
    type Future = BoxFuture<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: http::Request<ReqBody>) -> Self::Future {
        // poll_ready で準備ができたのは self.inner なので、そちらを持ち出して代わりに複製を残す
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        let method = RpcMethod::from_path(request.uri().path());
        let excluded = method.as_ref().is_some_and(|method| {
            let full_name = method.full_name();
            self.excluded_rpcs
                .iter()
                .any(|rpc| *rpc == method.service() || *rpc == full_name)
        });
        let span = rpc_span(method.as_ref(), request.headers());
        // future が途中で捨てられた(クライアントの切断・期限切れ)ときも、drop でアクセスログを出す
        let mut log = AccessLog::new(span.clone(), excluded);

        let future = async move {
            match AssertUnwindSafe(inner.call(request)).catch_unwind().await {
                Ok(Ok(response)) => {
                    log.read_status(response.headers());
                    Ok(response.map(|body| AccessLogBody::new(body, log)))
                }
                Ok(Err(err)) => {
                    log.set_status_if_missing(Code::Unknown);
                    Err(err)
                }
                Err(panic) => {
                    // tokio はタスクの panic を捕まえてから future を捨てるので、drop の時点では panic と見分けられない。ここで捕まえる
                    log.panicked = true;
                    let response = voltech_grpc_error::internal(Internal::msg(format!(
                        "handler panicked: {}",
                        panic_message(panic.as_ref())
                    )))
                    .into_http::<ResBody>();
                    log.read_status(response.headers());
                    Ok(response.map(|body| AccessLogBody::new(body, log)))
                }
            }
        };
        Box::pin(future.instrument(span))
    }
}

fn rpc_span(method: Option<&RpcMethod>, headers: &HeaderMap) -> Span {
    let rpc_method = method.map_or_else(|| "_OTHER".to_owned(), RpcMethod::full_name);
    // 名前は field の定数と同じにする(後から書き込む側は定数を使う)
    let span = tracing::info_span!(
        target: RPC_SPAN_TARGET,
        "rpc",
        "rpc.system.name" = "grpc",
        "rpc.method" = rpc_method,
        request_id = Empty,
        player_id = Empty,
        replayed = Empty,
    );
    // メタデータは x-request-id しか読まない(authorization などをログに出さないため)
    let request_id = headers
        .get(REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .filter(|request_id| is_valid_request_id(request_id));
    if let Some(request_id) = request_id {
        span.record(field::REQUEST_ID, request_id);
    }
    span
}

fn panic_message(panic: &(dyn Any + Send)) -> &str {
    if let Some(message) = panic.downcast_ref::<&str>() {
        message
    } else if let Some(message) = panic.downcast_ref::<String>() {
        message
    } else {
        "unknown panic payload"
    }
}

/// 1つの RPC のアクセスログ。出すのは1回だけで、出さないまま捨てられたら drop で出す。
struct AccessLog {
    span: Span,
    started: Instant,
    excluded: bool,
    status: Option<RpcStatus>,
    panicked: bool,
    emitted: bool,
}

struct RpcStatus {
    code: Code,
    /// 業務エラーの `ErrorInfo` の domain と reason。
    error_info: Option<(String, String)>,
}

impl AccessLog {
    fn new(span: Span, excluded: bool) -> Self {
        Self {
            span,
            started: Instant::now(),
            excluded,
            status: None,
            panicked: false,
            emitted: false,
        }
    }

    /// gRPC のステータスを読む。成功したときは trailers に、Trailers-Only のエラーは headers に入っている。
    fn read_status(&mut self, headers: &HeaderMap) {
        if self.status.is_some() {
            return;
        }
        let Some(value) = headers.get("grpc-status") else {
            return;
        };
        let code = Code::from_bytes(value.as_bytes());
        // ErrorInfo はエラーのときだけ読む(成功したときに grpc-status-details-bin を解かない)
        let error_info = (code != Code::Ok)
            .then(|| Status::from_header_map(headers))
            .flatten()
            .and_then(|status| {
                status
                    .get_error_details()
                    .error_info()
                    .map(|info| (info.domain.clone(), info.reason.clone()))
            });
        self.status = Some(RpcStatus { code, error_info });
    }

    fn set_status_if_missing(&mut self, code: Code) {
        if self.status.is_none() {
            self.status = Some(RpcStatus {
                code,
                error_info: None,
            });
        }
    }

    fn emit(&mut self) {
        if std::mem::replace(&mut self.emitted, true) || self.excluded {
            return;
        }
        // ステータスを返す前に終わったのは、クライアントが切断したか期限が切れたとき
        let (code, error_info) = match &self.status {
            Some(status) => (status.code, status.error_info.as_ref()),
            None => (Code::Cancelled, None),
        };
        let status_code = code_name(code);
        let error_type = is_server_error(code).then_some(status_code);
        let (domain, reason) = error_info
            .map(|(domain, reason)| (domain.as_str(), reason.as_str()))
            .unzip();
        let panic = self.panicked.then_some(true);
        let duration_ms = self.started.elapsed().as_secs_f64() * 1000.0;

        let _entered = self.span.enter();
        macro_rules! access_log {
            ($level:expr) => {
                tracing::event!(
                    target: ACCESS_LOG_TARGET,
                    $level,
                    "rpc.response.status_code" = status_code,
                    "error.type" = error_type,
                    "error.domain" = domain,
                    "error.reason" = reason,
                    panic,
                    duration_ms,
                    "rpc finished"
                )
            };
        }
        // 業務エラー(FAILED_PRECONDITION など)は error にしない。サーバーの不具合と見分けるため
        if self.panicked || matches!(code, Code::Unknown | Code::Internal | Code::DataLoss) {
            access_log!(Level::ERROR);
        } else {
            access_log!(Level::INFO);
        }
    }
}

impl Drop for AccessLog {
    fn drop(&mut self) {
        self.emit();
    }
}

// OpenTelemetry の gRPC の決まりで、サーバー側のエラーとして error.type を付けるステータス
fn is_server_error(code: Code) -> bool {
    matches!(
        code,
        Code::Unknown
            | Code::DeadlineExceeded
            | Code::Unimplemented
            | Code::Internal
            | Code::Unavailable
            | Code::DataLoss
    )
}

/// gRPC のステータスの名前(`.proto` の `google.rpc.Code` と同じ書き方)。
fn code_name(code: Code) -> &'static str {
    match code {
        Code::Ok => "OK",
        Code::Cancelled => "CANCELLED",
        Code::Unknown => "UNKNOWN",
        Code::InvalidArgument => "INVALID_ARGUMENT",
        Code::DeadlineExceeded => "DEADLINE_EXCEEDED",
        Code::NotFound => "NOT_FOUND",
        Code::AlreadyExists => "ALREADY_EXISTS",
        Code::PermissionDenied => "PERMISSION_DENIED",
        Code::ResourceExhausted => "RESOURCE_EXHAUSTED",
        Code::FailedPrecondition => "FAILED_PRECONDITION",
        Code::Aborted => "ABORTED",
        Code::OutOfRange => "OUT_OF_RANGE",
        Code::Unimplemented => "UNIMPLEMENTED",
        Code::Internal => "INTERNAL",
        Code::Unavailable => "UNAVAILABLE",
        Code::DataLoss => "DATA_LOSS",
        Code::Unauthenticated => "UNAUTHENTICATED",
    }
}

pin_project! {
    /// `RpcLogLayer` が返すレスポンスの body。trailers のステータスを読み、body が終わったらアクセスログを出す。
    pub struct AccessLogBody<B> {
        #[pin]
        inner: B,
        log: Option<AccessLog>,
    }
}

impl<B> AccessLogBody<B> {
    fn new(inner: B, log: AccessLog) -> Self {
        Self {
            inner,
            log: Some(log),
        }
    }
}

impl<B: Default> Default for AccessLogBody<B> {
    fn default() -> Self {
        Self {
            inner: B::default(),
            log: None,
        }
    }
}

impl<B: Body> Body for AccessLogBody<B> {
    type Data = B::Data;
    type Error = B::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let this = self.project();
        let frame = ready!(this.inner.poll_frame(cx));
        match &frame {
            Some(Ok(frame)) => {
                if let (Some(trailers), Some(log)) = (frame.trailers_ref(), this.log.as_mut()) {
                    log.read_status(trailers);
                }
            }
            Some(Err(_)) => {
                if let Some(mut log) = this.log.take() {
                    log.set_status_if_missing(Code::Unknown);
                }
            }
            // 終わった時点で出す(処理時間に、body を捨てるまでの時間を含めない)
            None => drop(this.log.take()),
        }
        Poll::Ready(frame)
    }

    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.inner.size_hint()
    }
}
