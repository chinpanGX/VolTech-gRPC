mod common;

use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::Value;
use tracing::subscriber::DefaultGuard;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::SubscriberExt;
use voltech_grpc_auth::PlayerStatus;

use common::{SECRET_KEY, TestServer, start};

/// ログの出力をためておく書き込み先。
#[derive(Clone, Default)]
struct Output(Arc<Mutex<Vec<u8>>>);

impl Output {
    fn text(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }

    fn lines(&self) -> Vec<Value> {
        self.text()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }

    /// `message` のログを返す。
    fn find(&self, message: &str) -> Vec<Value> {
        self.lines()
            .into_iter()
            .filter(|line| line["message"] == message)
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
static LOGGING: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// 基盤のログの形式(JSON)で、テストの間だけ `output` へ出す。tokio::test は1つのスレッドで動くので、サーバーのログも受け取れる。
fn capture(output: &Output) -> DefaultGuard {
    let config = voltech_grpc_log::LogConfig::new("test-server", "test");
    let subscriber = tracing_subscriber::registry()
        .with(voltech_grpc_log::layer_with_writer(&config, output.clone()).unwrap());
    tracing::subscriber::set_default(subscriber)
}

/// 登録・発行・認証のログに player_id が付き、発行したトークンと secret_key はどのログにも出ない。
#[tokio::test]
async fn token_is_never_logged_and_player_id_is_recorded() {
    let _serial = LOGGING.lock().await;
    let output = Output::default();
    let _guard = capture(&output);
    let server = start().await;

    let registered = server.register_response(SECRET_KEY).await.unwrap();
    let token = server
        .issue_token(&registered.device_id, SECRET_KEY)
        .await
        .unwrap();
    server.call_protected(Some(&token)).await.unwrap();

    let text = output.text();
    assert!(!text.contains(&token), "{text}");
    assert!(!text.contains(SECRET_KEY), "{text}");

    let registered_log = &output.find("device registered")[0];
    assert_eq!(registered_log["device_id"], registered.device_id);
    assert_eq!(registered_log["player_id"], registered.player_id);
    let issued_log = &output.find("access token issued")[0];
    assert_eq!(issued_log["player_id"], registered.player_id);
    // 認証が必要なサービスのアクセスログにも、AuthLayer が書いた player_id が付く
    let access_logs: Vec<Value> = output
        .lines()
        .into_iter()
        .filter(|line| line["target"] == voltech_grpc_log::ACCESS_LOG_TARGET)
        .collect();
    assert_eq!(access_logs.len(), 3, "{access_logs:?}");
    assert!(
        access_logs
            .iter()
            .all(|line| line["player_id"] == registered.player_id),
        "{access_logs:?}"
    );
}

/// 認証の失敗は、トークンが無ければ warn、期限切れ・知らないトークンなら info で、理由を付けて出す。
#[tokio::test]
async fn authentication_failures_are_logged_with_reason() {
    let _serial = LOGGING.lock().await;
    let output = Output::default();
    let _guard = capture(&output);
    let server = TestServer::start(PlayerStatus::Active, Duration::ZERO).await;
    let device_id = server.register().await;
    let expired = server.issue_token(&device_id, SECRET_KEY).await.unwrap();

    let _ = server.call_protected(None).await;
    let _ = server.call_protected(Some(&expired)).await;
    let _ = server.call_protected(Some("unknown")).await;

    let failures: Vec<(Value, Value)> = output
        .find("authentication failed")
        .into_iter()
        .map(|line| (line["reason"].clone(), line["level"].clone()))
        .collect();
    assert_eq!(
        failures,
        [
            ("missing_token".into(), "WARN".into()),
            ("expired_token".into(), "INFO".into()),
            ("unknown_token".into(), "INFO".into()),
        ]
    );
    assert!(!output.text().contains(&expired));
}

/// トークンの発行の失敗は、知らないデバイス・違う secret_key のどちらも warn で、理由を付けて出す。
#[tokio::test]
async fn issue_token_failures_are_warned() {
    let _serial = LOGGING.lock().await;
    let output = Output::default();
    let _guard = capture(&output);
    let server = start().await;
    let device_id = server.register().await;

    let _ = server.issue_token("unknown-device", SECRET_KEY).await;
    let _ = server
        .issue_token(&device_id, "fedcba9876543210fedcba9876543210")
        .await;

    let failures: Vec<(Value, Value)> = output
        .find("access token issue failed")
        .into_iter()
        .map(|line| (line["reason"].clone(), line["level"].clone()))
        .collect();
    assert_eq!(
        failures,
        [
            ("unknown_device".into(), "WARN".into()),
            ("invalid_secret".into(), "WARN".into()),
        ]
    );
}
