use std::sync::{Arc, RwLock};
use std::time::Duration;

use jiff::Timestamp;
use jiff::civil::Date;

use crate::BoxError;
use crate::config::{Maintenance, StatusConfig};
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
                        let mut current = config.write().unwrap_or_else(|e| e.into_inner());
                        log_maintenance_change(&current.maintenance, &loaded.maintenance);
                        *current = Arc::new(loaded);
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

fn log_maintenance_change(previous: &Maintenance, loaded: &Maintenance) {
    match (previous.enabled, loaded.enabled) {
        (false, true) => {
            let end_at = loaded.end_at.map(tracing::field::display);
            tracing::info!(end_at, "maintenance started");
        }
        (true, false) => tracing::info!("maintenance ended"),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use tracing_subscriber::fmt::MakeWriter;

    use super::*;

    #[derive(Clone, Default)]
    struct Output(Arc<Mutex<Vec<u8>>>);

    impl std::io::Write for Output {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for Output {
        type Writer = Self;

        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    fn maintenance(enabled: bool) -> Maintenance {
        Maintenance {
            enabled,
            end_at: enabled.then(|| "2026-10-06T06:00:00Z".parse().unwrap()),
        }
    }

    /// メンテナンスが始まった・終わったときだけ info で出し、変わらなければ何も出さない。
    #[test]
    fn logs_only_maintenance_switch() {
        let output = Output::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(output.clone())
            .with_ansi(false)
            .finish();
        tracing::subscriber::with_default(subscriber, || {
            log_maintenance_change(&maintenance(false), &maintenance(true));
            log_maintenance_change(&maintenance(true), &maintenance(true));
            log_maintenance_change(&maintenance(true), &maintenance(false));
            log_maintenance_change(&maintenance(false), &maintenance(false));
        });

        let logs = String::from_utf8(output.0.lock().unwrap().clone()).unwrap();
        let lines: Vec<&str> = logs.lines().collect();
        assert_eq!(lines.len(), 2, "{logs}");
        assert!(lines[0].contains(" INFO "), "{logs}");
        assert!(lines[0].contains("maintenance started"), "{logs}");
        assert!(lines[0].contains("end_at=2026-10-06T06:00:00Z"), "{logs}");
        assert!(lines[1].contains("maintenance ended"), "{logs}");
    }
}
