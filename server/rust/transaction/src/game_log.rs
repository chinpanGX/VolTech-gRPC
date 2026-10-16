use sqlx::{MySql, Transaction};
use ulid::Ulid;
use voltech_grpc_log::GAME_LOG_TARGET;

/// ゲームログ(ガチャでのジェムの消費のような、所持データの変化)の1件。問い合わせ・補填・分析に使う。
///
/// `TxContext::write_game_log` で書く。`player_id`・`request_id`・RPC・時刻は基盤が付ける。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameLog {
    kind: String,
    target_id: Option<String>,
    delta: i64,
    after: i64,
    reason: String,
}

impl GameLog {
    /// 種別(例: `gems`・`item`)・増減・更新後の値・理由(例: `gacha_draw`)で作る。
    pub fn new(kind: impl Into<String>, delta: i64, after: i64, reason: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            target_id: None,
            delta,
            after,
            reason: reason.into(),
        }
    }

    /// 対象の ID(アイテムの ID など)を付ける。名前は変わることがあるので、ID を書く。
    pub fn target(mut self, target_id: impl ToString) -> Self {
        self.target_id = Some(target_id.to_string());
        self
    }
}

/// ゲームログの出力先。`Transactions::new` に渡して選ぶ。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameLogSink {
    /// DB の `game_logs` テーブル。業務の処理と同じトランザクションで書くので、ロールバックしたら残らない。
    Database,
    /// 標準出力の JSON(target は `voltech_grpc_log::GAME_LOG_TARGET`、`log_type` は `game`)。
    /// ログの収集・分析のサービスへ送る構成向け。コミットに成功してから出すので、直後にプロセスが落ちると残らない。
    Stdout,
}

/// ゲームログに基盤が付ける値。
pub(crate) struct GameLogOrigin {
    pub(crate) player_id: Ulid,
    pub(crate) request_id: Option<String>,
    pub(crate) method: String,
}

pub(crate) async fn write_to_database(
    tx: &mut Transaction<'static, MySql>,
    origin: &GameLogOrigin,
    logs: &[GameLog],
) -> sqlx::Result<()> {
    for log in logs {
        sqlx::query(
            "INSERT INTO game_logs (player_id, request_id, method, kind, body) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(origin.player_id.to_bytes().as_slice())
        .bind(origin.request_id.as_deref())
        .bind(&origin.method)
        .bind(&log.kind)
        .bind(body(log).to_string())
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

pub(crate) fn write_to_stdout(origin: &GameLogOrigin, logs: &[GameLog]) {
    for log in logs {
        tracing::info!(
            target: GAME_LOG_TARGET,
            log_type = "game",
            player_id = %origin.player_id,
            request_id = origin.request_id.as_deref(),
            "rpc.method" = origin.method,
            kind = log.kind,
            target_id = log.target_id.as_deref(),
            delta = log.delta,
            after = log.after,
            reason = log.reason,
            "game log"
        );
    }
}

// 種別ごとにテーブルを分けないので、種別で形の変わりうる値は JSON にまとめる
fn body(log: &GameLog) -> serde_json::Value {
    serde_json::json!({
        "target_id": log.target_id,
        "delta": log.delta,
        "after": log.after,
        "reason": log.reason,
    })
}
