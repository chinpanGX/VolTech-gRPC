use std::task::{Context, Poll};

use tower::{Layer, Service};
use voltech_grpc_log::RpcMethod;

/// リクエストの HTTP のパスから RPC の名前を取り出し、`RpcMethod` として extensions に入れる tower の Layer。
///
/// tonic はサーバー側のハンドラーに RPC の名前を渡さないので、`Transactions::run` のためにルーターの外側に付ける。
#[derive(Clone, Copy, Debug, Default)]
pub struct RpcMethodLayer;

impl<S> Layer<S> for RpcMethodLayer {
    type Service = RpcMethodService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RpcMethodService { inner }
    }
}

/// `RpcMethodLayer` を付けたサービス。
#[derive(Clone, Debug)]
pub struct RpcMethodService<S> {
    inner: S,
}

impl<S, B> Service<http::Request<B>> for RpcMethodService<S>
where
    S: Service<http::Request<B>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut request: http::Request<B>) -> Self::Future {
        if let Some(method) = RpcMethod::from_path(request.uri().path()) {
            request.extensions_mut().insert(method);
        }
        self.inner.call(request)
    }
}
