use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;

use crate::BoxError;

/// 基盤のテストが使う DB(app)とは分ける。
const DEFAULT_DATABASE_URL: &str = "mysql://app:app@localhost:13306/voltech_sandbox";
const DEFAULT_LISTEN_ADDR: &str = "0.0.0.0:50051";
// make sandbox-server はリポジトリのルートで動かすので、ルートからのパスにする
const DEFAULT_SERVER_STATUS_PATH: &str = "sandbox/server/config/server_status.toml";

/// 環境変数から読む、サーバーの設定。
#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub listen_addr: SocketAddr,
    /// サーバーの状態(メンテナンスなど)の設定ファイル。
    pub server_status_path: PathBuf,
    /// gRPC リフレクションを有効にするか。開発環境(`APP_ENV=development`)だけ有効にする。
    pub enable_reflection: bool,
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
        let enable_reflection = env::var("APP_ENV").is_ok_and(|value| value == "development");

        Ok(Self {
            database_url,
            listen_addr,
            server_status_path,
            enable_reflection,
        })
    }
}
