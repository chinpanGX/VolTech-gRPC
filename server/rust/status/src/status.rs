use std::sync::{Arc, RwLock};
use std::time::Duration;

use jiff::Timestamp;
use jiff::civil::Date;

use crate::BoxError;
use crate::config::StatusConfig;
use crate::source::StatusSource;

/// 今の時刻を返す。テストでは決まった時刻を返すものに差し替える。
pub trait Clock: Send + Sync + 'static {
    fn now(&self) -> Timestamp;
}

/// サーバーの時計。
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        Timestamp::now()
    }
}

/// 今のサーバーの状態。Layer はこれを見て判定する。複製しても同じ状態を指す。
#[derive(Clone)]
pub struct ServerStatus {
    config: Arc<RwLock<Arc<StatusConfig>>>,
    clock: Arc<dyn Clock>,
}

impl ServerStatus {
    /// 決まった設定で作る。読み直しはしない(テストや、設定を変えない環境で使う)。
    pub fn new(config: StatusConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(Arc::new(config))),
            clock: Arc::new(SystemClock),
        }
    }

    /// `source` から設定を読み、以後 `interval` ごとに読み直す。
    ///
    /// 最初の読み込みに失敗したらエラーを返す(サーバーは起動しない)。
    /// 読み直しに失敗したときは、ログに出して直前の設定を使い続ける。
    /// 読み直しは、この `ServerStatus` の複製がすべて捨てられると止まる。
    pub async fn watch<S: StatusSource>(source: S, interval: Duration) -> Result<Self, BoxError> {
        let status = Self::new(source.load().await?);
        let weak = Arc::downgrade(&status.config);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;
                let Some(config) = weak.upgrade() else {
                    break;
                };
                match source.load().await {
                    Ok(loaded) => {
                        *config.write().unwrap_or_else(|e| e.into_inner()) = Arc::new(loaded)
                    }
                    Err(err) => {
                        tracing::warn!(%err, "failed to reload server status; keep the previous one")
                    }
                }
            }
        });
        Ok(status)
    }

    /// 時計を差し替える。
    pub fn with_clock(mut self, clock: impl Clock) -> Self {
        self.clock = Arc::new(clock);
        self
    }

    /// 今の設定。
    pub fn config(&self) -> Arc<StatusConfig> {
        self.config
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// 今の時刻。
    pub fn now(&self) -> Timestamp {
        self.clock.now()
    }

    /// 今のゲーム内の日付。
    pub fn game_date(&self) -> Date {
        self.config().game_day.date_at(self.now())
    }
}
