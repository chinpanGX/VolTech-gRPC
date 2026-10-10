use std::task::{Context, Poll};

use tower::{Layer, Service};

/// 呼ばれた RPC の名前。`RpcMethodLayer` がリクエストの extensions に入れる。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RpcMethod {
    service: String,
    method: String,
}

impl RpcMethod {
    /// gRPC の HTTP のパス(例: /app.v1.GachaService/Draw)から作る。形が違えば `None` を返す。
    pub fn from_path(path: &str) -> Option<Self> {
        let (service, method) = path.strip_prefix('/')?.split_once('/')?;
        if service.is_empty() || method.is_empty() {
            return None;
        }
        Some(Self {
            service: service.to_owned(),
            method: method.to_owned(),
        })
    }

    /// パッケージ付きのサービス名(例: app.v1.GachaService)。
    pub fn service(&self) -> &str {
        &self.service
    }

    /// メソッド名(例: Draw)。
    pub fn method(&self) -> &str {
        &self.method
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    /// gRPC のパスからサービス名とメソッド名を取り出し、形が違うパスは取り出さない。
    #[test]
    fn from_path_splits_service_and_method() {
        let method = RpcMethod::from_path("/app.v1.GachaService/Draw").unwrap();
        assert_eq!(method.service(), "app.v1.GachaService");
        assert_eq!(method.method(), "Draw");

        for path in [
            "",
            "/",
            "app.v1.GachaService/Draw",
            "/app.v1.GachaService",
            "/app.v1.GachaService/",
        ] {
            assert_eq!(RpcMethod::from_path(path), None, "{path}");
        }
    }
}
