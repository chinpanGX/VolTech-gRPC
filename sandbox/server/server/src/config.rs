use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;

use voltech_grpc_log::LogConfig;
use voltech_grpc_transaction::GameLogSink;

use crate::BoxError;

/// 基盤のテストが使う DB(app)とは分ける。
const DEFAULT_DATABASE_URL: &str = "mysql://app:app@localhost:13306/voltech_sandbox";
const DEFAULT_LISTEN_ADDR: &str = "0.0.0.0:50051";
// make sandbox-server はリポジトリのルートで動かすので、ルートからのパスにする
const DEFAULT_SERVER_STATUS_PATH: &str = "sandbox/server/config/server_status.toml";
const DEFAULT_SERVICE_NAME: &str = "sandbox-server";

/// 環境変数から読む、サーバーの設定。
#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub listen_addr: SocketAddr,
    /// サーバーの状態(メンテナンスなど)の設定ファイル。
    pub server_status_path: PathBuf,
    /// gRPC リフレクションを有効にするか。開発環境(`APP_ENV=development`)だけ有効にする。
    pub enable_reflection: bool,
    /// ログの設定。
    pub log: LogConfig,
    /// ゲームログの出力先。
    pub game_log_sink: GameLogSink,
}

impl Config {
    /// 環境変数から設定を読む。DB は `SANDBOX_DATABASE_URL`(無ければ voltech_sandbox)。
    pub fn from_env() -> Result<Self, BoxError> {
        let database_url =
            env::var("SANDBOX_DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_owned());
        let listen_addr = env::var("LISTEN_ADDR")
            .unwrap_or_else(|_| DEFAULT_LISTEN_ADDR.to_owned())
            .parse()?;
        let server_status_path = env::var("SERVER_STATUS_PATH")
            .unwrap_or_else(|_| DEFAULT_SERVER_STATUS_PATH.to_owned())
            .into();
        let app_env = env::var("APP_ENV").ok();
        let enable_reflection = app_env.as_deref() == Some("development");
        let log = log_config_from_env(app_env.as_deref().unwrap_or("unknown"))?;
        let game_log_sink = game_log_sink_from_env()?;

        Ok(Self {
            database_url,
            listen_addr,
            server_status_path,
            enable_reflection,
            log,
            game_log_sink,
        })
    }
}

/// ログの設定を環境変数から読む。無い項目は基盤の既定の値にする。
///
/// - `LOG_FORMAT`: `json`(既定)か `text`
/// - `RUST_LOG`: レベルのフィルター(既定は `info`)
/// - `LOG_FIELD_NAMES`: JSON の項目名。`standard`(既定)か `gcp`
/// - `OTEL_SERVICE_NAME`: サービス名(既定は `sandbox-server`)
/// - 環境名は `APP_ENV` を使う
fn log_config_from_env(environment: &str) -> Result<LogConfig, BoxError> {
    let service_name =
        env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| DEFAULT_SERVICE_NAME.to_owned());
    let mut log = LogConfig::new(service_name, environment);
    if let Ok(format) = env::var("LOG_FORMAT") {
        log.format = format.parse()?;
    }
    if let Ok(filter) = env::var("RUST_LOG") {
        log.filter = filter;
    }
    if let Ok(field_names) = env::var("LOG_FIELD_NAMES") {
        log.field_names = field_names.parse()?;
    }
    Ok(log)
}

/// ゲームログの出力先を `GAME_LOG_SINK` から読む。`database`(既定)か `stdout`。
fn game_log_sink_from_env() -> Result<GameLogSink, BoxError> {
    match env::var("GAME_LOG_SINK").as_deref() {
        Err(_) | Ok("database") => Ok(GameLogSink::Database),
        Ok("stdout") => Ok(GameLogSink::Stdout),
        // 書き間違いのまま別の出力先で動き続けないように、起動を止める
        Ok(other) => Err(format!(
            "unknown GAME_LOG_SINK {other:?} (expected one of: database, stdout)"
        )
        .into()),
    }
}
