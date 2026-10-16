//! ログの出力を読むための書き込み先と、テストで使うレスポンスの body。

#![allow(
    dead_code,
    reason = "テストのファイルごとに使う関数が違い、使わないファイルでは dead_code になるため"
)]

use std::collections::VecDeque;
use std::convert::Infallible;
use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use bytes::Bytes;
use http::HeaderMap;
use http_body::{Body, Frame};
use serde_json::Value;
use tracing::Subscriber;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::SubscriberExt;
use voltech_grpc_log::LogConfig;

/// ログの出力をためておく書き込み先。
#[derive(Clone, Default)]
pub struct Output(Arc<Mutex<Vec<u8>>>);

impl Output {
    /// ためた出力をそのまま返す。
    pub fn text(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }

    /// ためた出力を、1行1件の JSON として読む。
    pub fn lines(&self) -> Vec<Value> {
        self.text()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }

    /// アクセスログの行だけを返す。
    pub fn access_logs(&self) -> Vec<Value> {
        self.lines()
            .into_iter()
            .filter(|line| line["target"] == voltech_grpc_log::ACCESS_LOG_TARGET)
            .collect()
    }
}

impl io::Write for Output {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for Output {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

/// サービス名 `test-server`、環境名 `test` の設定。
pub fn config() -> LogConfig {
    LogConfig::new("test-server", "test")
}

/// `config` どおりに `output` へ出す subscriber を作る。
pub fn subscriber(config: &LogConfig, output: &Output) -> impl Subscriber + Send + Sync {
    tracing_subscriber::registry()
        .with(voltech_grpc_log::layer_with_writer(config, output.clone()).unwrap())
}

/// 決まったフレームを順に返す body。
#[derive(Default)]
pub struct TestBody(VecDeque<Frame<Bytes>>);

impl TestBody {
    /// データの後に、`grpc-status: 0` の trailers を返す(成功した unary の RPC と同じ形)。
    pub fn ok() -> Self {
        let mut trailers = HeaderMap::new();
        trailers.insert("grpc-status", "0".parse().unwrap());
        Self(VecDeque::from([
            Frame::data(Bytes::from_static(b"\0\0\0\0\0")),
            Frame::trailers(trailers),
        ]))
    }
}

impl Body for TestBody {
    type Data = Bytes;
    type Error = Infallible;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        Poll::Ready(self.0.pop_front().map(Ok))
    }

    fn is_end_stream(&self) -> bool {
        self.0.is_empty()
    }
}
