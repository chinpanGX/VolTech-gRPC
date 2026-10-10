use std::future::Future;
use std::path::PathBuf;

use crate::BoxError;
use crate::config::StatusConfig;

/// サーバーの状態の設定の読み込み元。AWS に載せるときは、AWS の設定サービスから読む実装に差し替える。
pub trait StatusSource: Send + Sync + 'static {
    /// 今の設定を読む。
    fn load(&self) -> impl Future<Output = Result<StatusConfig, BoxError>> + Send;
}

/// TOML のファイル(`server_status.toml`)から読む。
#[derive(Clone, Debug)]
pub struct FileStatusSource {
    path: PathBuf,
}

impl FileStatusSource {
    /// `path` のファイルから読む読み込み元を作る。
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl StatusSource for FileStatusSource {
    async fn load(&self) -> Result<StatusConfig, BoxError> {
        let text = tokio::fs::read_to_string(&self.path)
            .await
            .map_err(|err| format!("{}: {err}", self.path.display()))?;
        StatusConfig::from_toml(&text)
            .map_err(|err| format!("{}: {err}", self.path.display()).into())
    }
}
