//! テストで使う DB の接続と、RPC の名前・ヘッダーを入れたリクエストの作り方。

#![allow(
    dead_code,
    reason = "テストのファイルごとに使う関数が違い、使わないファイルでは dead_code になるため"
)]

use std::io;
use std::sync::{Arc, Mutex};

use prost::Message;
use prost_types::method_options::IdempotencyLevel;
use prost_types::{
    FileDescriptorProto, FileDescriptorSet, MethodDescriptorProto, MethodOptions,
    ServiceDescriptorProto,
};
use serde_json::Value;
use sqlx::MySqlPool;
use tonic::Request;
use tracing::Subscriber;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::SubscriberExt;
use ulid::Ulid;
use voltech_grpc_auth::AuthContext;
use voltech_grpc_transaction::{GameLogSink, IdempotencyLevels, RpcMethod, Transactions};

const DEFAULT_DATABASE_URL: &str = "mysql://app:app@localhost:13306/app";

/// テスト用のサービスの名前。
pub const SERVICE: &str = "test.v1.SampleService";
/// 冪等性の対象の RPC(idempotency_level を書いていない)。
pub const DRAW: &str = "Draw";
/// 冪等性の対象の、もう1つの RPC。
pub const BUY: &str = "Buy";
/// `NO_SIDE_EFFECTS` の RPC。
pub const READ: &str = "Read";
/// `IDEMPOTENT` の RPC。
pub const SIGN_IN: &str = "SignIn";

/// マイグレーションを実行し、テスト用のサービスの表を持った、ゲームログを DB に書く `Transactions` を作る。
pub async fn start() -> (Transactions, MySqlPool) {
    start_with_sink(GameLogSink::Database).await
}

/// `start` と同じだが、ゲームログの出力先を選ぶ。
pub async fn start_with_sink(sink: GameLogSink) -> (Transactions, MySqlPool) {
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_owned());
    let pool = MySqlPool::connect(&database_url).await.unwrap();
    voltech_grpc_transaction::migrate(&pool).await.unwrap();

    let levels = IdempotencyLevels::from_file_descriptor_set(&sample_descriptor_set()).unwrap();
    (Transactions::new(pool.clone(), levels, sink), pool)
}

/// RPC の名前と、`x-request-id`(`None` なら付けない)を入れたリクエストを作る。
pub fn request(method: &'static str, request_id: Option<&str>) -> Request<()> {
    let mut request = Request::new(());
    request
        .extensions_mut()
        .insert(RpcMethod::from_path(&format!("/{SERVICE}/{method}")).unwrap());
    if let Some(request_id) = request_id {
        request
            .metadata_mut()
            .insert("x-request-id", request_id.parse().unwrap());
    }
    request
}

/// `request` に、`AuthLayer` が `player_id` を認証したときと同じ `AuthContext` を入れる。
pub fn authenticated(mut request: Request<()>, player_id: Ulid) -> Request<()> {
    request.extensions_mut().insert(AuthContext {
        device_id: Ulid::generate().to_string(),
        player_id,
        player_created: true,
    });
    request
}

fn sample_descriptor_set() -> Vec<u8> {
    let method = |name: &str, level: Option<IdempotencyLevel>| MethodDescriptorProto {
        name: Some(name.to_owned()),
        options: level.map(|level| MethodOptions {
            idempotency_level: Some(level as i32),
            ..Default::default()
        }),
        ..Default::default()
    };
    FileDescriptorSet {
        file: vec![FileDescriptorProto {
            package: Some("test.v1".to_owned()),
            service: vec![ServiceDescriptorProto {
                name: Some("SampleService".to_owned()),
                method: vec![
                    method(DRAW, None),
                    method(BUY, None),
                    method(READ, Some(IdempotencyLevel::NoSideEffects)),
                    method(SIGN_IN, Some(IdempotencyLevel::Idempotent)),
                ],
                ..Default::default()
            }],
            ..Default::default()
        }],
    }
    .encode_to_vec()
}

/// ログの出力をためておく書き込み先。
#[derive(Clone, Default)]
pub struct Output(Arc<Mutex<Vec<u8>>>);

impl Output {
    /// ためた出力を、1行1件の JSON として読む。
    pub fn lines(&self) -> Vec<Value> {
        String::from_utf8(self.0.lock().unwrap().clone())
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
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

/// ログを読むテストを1つずつ動かすためのロック。
///
/// tracing は、ログを出す場所ごとに「出すか」をプロセス全体で覚えておく。テストごとの subscriber を
/// 別のスレッドで同時に作ったり捨てたりすると、それがずれてログが落ちることがあるため、同時に動かさない。
pub static LOGGING: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// 基盤のログの形式(JSON)で `output` へ出す subscriber を作る。
pub fn subscriber(output: &Output) -> impl Subscriber + Send + Sync {
    let config = voltech_grpc_log::LogConfig::new("test-server", "test");
    tracing_subscriber::registry()
        .with(voltech_grpc_log::layer_with_writer(&config, output.clone()).unwrap())
}
