use serde_json::{Map, Value};
use sqlx::{MySql, Transaction};
use ulid::Ulid;
use voltech_grpc_error::Internal;
use voltech_grpc_log::{GAME_LOG_FIELDS, GAME_LOG_TARGET, is_valid_request_id};

/// 値の名前に使えない名前。基盤がゲームログ・ログの1行に付ける項目と同じ名前にすると、どちらかが読めなくなるため。
const RESERVED_FIELD_NAMES: &[&str] = &[
    // ゲームログの項目
    "kind",
    "reason",
    "target_id",
    "delta",
    "after",
    "player_id",
    "request_id",
    "rpc.method",
    "log_type",
    GAME_LOG_FIELDS,
    // ログの1行に基盤が付ける項目
    "timestamp",
    "time",
    "level",
    "severity",
    "message",
    "target",
    "service.name",
    "deployment.environment.name",
];
const MAX_FIELD_NAME_LEN: usize = 64;
// game_logs.method の長さ
const MAX_METHOD_LEN: usize = 255;

/// ゲームログ(ガチャでのジェムの消費・キャラクターの入手・編成の変更のような、所持データの変化)の1件。
/// 問い合わせ・補填・分析に使う。
///
/// 数量の変化は `GameLog::new`、数量の変化でないものは `GameLog::event` で作り、
/// 名前付きの値(個体の ID・枠など)を `field` で足す。`player_id`・`request_id`・RPC・時刻は基盤が付ける。
#[derive(Clone, Debug, PartialEq)]
pub struct GameLog {
    kind: String,
    reason: String,
    target_id: Option<String>,
    quantity: Option<Quantity>,
    fields: Vec<(String, Value)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Quantity {
    delta: i64,
    after: i64,
}

impl GameLog {
    /// 数量の変化を、種別(例: `gems`・`item`)・増減・更新後の値・理由(例: `gacha_draw`)で作る。
    pub fn new(kind: impl Into<String>, delta: i64, after: i64, reason: impl Into<String>) -> Self {
        Self {
            quantity: Some(Quantity { delta, after }),
            ..Self::event(kind, reason)
        }
    }

    /// 数量の変化でないもの(キャラクターの入手・編成の変更など)を、種別と理由で作る。`delta`・`after` は持たない。
    pub fn event(kind: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            reason: reason.into(),
            target_id: None,
            quantity: None,
            fields: Vec::new(),
        }
    }

    /// 対象の ID(アイテムの ID など)を付ける。名前は変わることがあるので、ID を書く。
    pub fn target(mut self, target_id: impl ToString) -> Self {
        self.target_id = Some(target_id.to_string());
        self
    }

    /// 名前付きの値(例: `unit_id`、編成の枠の配列)を足す。名前は英小文字・数字・`_`・`.` の1〜64文字。
    ///
    /// 基盤が付ける項目と同じ名前(`kind`・`player_id` など)は使えない。書くときにエラーになる。
    pub fn field(mut self, name: impl Into<String>, value: impl Into<Value>) -> Self {
        self.fields.push((name.into(), value.into()));
        self
    }

    #[track_caller]
    fn validate(&self) -> Result<(), Internal> {
        for (index, (name, _)) in self.fields.iter().enumerate() {
            let valid_chars = name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'.');
            if name.is_empty() || name.len() > MAX_FIELD_NAME_LEN || !valid_chars {
                return Err(Internal::msg(format!(
                    "game log field name {name:?} must be 1..={MAX_FIELD_NAME_LEN} of [a-z0-9_.]"
                )));
            }
            if RESERVED_FIELD_NAMES.contains(&name.as_str()) {
                return Err(Internal::msg(format!(
                    "game log field name {name:?} is reserved"
                )));
            }
            if self.fields[..index].iter().any(|(other, _)| other == name) {
                return Err(Internal::msg(format!(
                    "game log field name {name:?} is duplicated"
                )));
            }
        }
        Ok(())
    }

    // 種別ごとにテーブルを分けないので、種別で形の変わりうる値は JSON にまとめる。無い値は入れない
    fn body(&self) -> Value {
        let mut body = Map::new();
        if let Some(target_id) = &self.target_id {
            body.insert("target_id".to_owned(), target_id.clone().into());
        }
        if let Some(quantity) = self.quantity {
            body.insert("delta".to_owned(), quantity.delta.into());
            body.insert("after".to_owned(), quantity.after.into());
        }
        body.insert("reason".to_owned(), self.reason.clone().into());
        for (name, value) in &self.fields {
            body.insert(name.clone(), value.clone());
        }
        Value::Object(body)
    }

    // 標準出力では、名前付きの値を1つの項目に JSON の文字列で入れ、ログの JSON の一番上の項目に展開する
    // (tracing のログの項目名は、コンパイル時に決まっていなければならないため)
    fn fields_json(&self) -> Option<String> {
        if self.fields.is_empty() {
            return None;
        }
        let fields: Map<String, Value> = self.fields.iter().cloned().collect();
        Some(Value::Object(fields).to_string())
    }
}

/// ゲームログの出力先。`Transactions::new` に渡して選ぶ。`Transactions::run` を通らない処理では `commit` を使う。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameLogSink {
    /// DB の `game_logs` テーブル。業務の処理と同じトランザクションで書くので、ロールバックしたら残らない。
    Database,
    /// 標準出力の JSON(target は `voltech_grpc_log::GAME_LOG_TARGET`、`log_type` は `game`)。
    /// ログの収集・分析のサービスへ送る構成向け。コミットに成功してから出すので、直後にプロセスが落ちると残らない。
    Stdout,
}

impl GameLogSink {
    /// `logs` を出力先へ書き、`tx` をコミットする。`Transactions::run` を通らない処理(サーバー間の内部 API など)で使う。
    ///
    /// `Database` なら `tx` の中で書いてからコミットし、`Stdout` ならコミットに成功してから出す。
    /// コミットせずに `tx` を捨てた(ロールバックした)ときは、何も残らない。
    pub async fn commit(
        self,
        mut tx: Transaction<'_, MySql>,
        logs: GameLogs,
    ) -> Result<(), Internal> {
        logs.validate()?;
        if self == Self::Database {
            write_to_database(&mut tx, &logs).await?;
        }
        tx.commit().await?;
        if self == Self::Stdout {
            write_to_stdout(&logs);
        }
        Ok(())
    }
}

/// コミットするときに書く、ゲームログの束。プレイヤーごとのゲームログと、それを書いた処理の名前を持つ。
///
/// `Transactions::run` を通らない処理で使う。`AuthContext` が無いので、変わる所持データの持ち主は `write` で渡す。
#[derive(Clone, Debug, PartialEq)]
pub struct GameLogs {
    method: String,
    request_id: Option<String>,
    entries: Vec<(Ulid, GameLog)>,
}

impl GameLogs {
    /// ゲームログを書いた処理の名前(`game_logs.method`)で作る。
    ///
    /// gRPC の RPC なら、`rpc.method` と同じ形(例: `internal.v1.MatchService/ReportResult`)にする。
    /// RPC でない処理(定期実行のジョブなど)は、`job/SeasonReward` のように見分けられる名前にする。ASCII の 1〜255 文字。
    pub fn new(method: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            request_id: None,
            entries: Vec::new(),
        }
    }

    /// `x-request-id` を付ける。形が正しく(英数字と `-` の1〜64文字)なければ付けない。
    pub fn with_request_id(mut self, request_id: &str) -> Self {
        self.request_id = is_valid_request_id(request_id).then(|| request_id.to_owned());
        self
    }

    /// `player_id` のプレイヤーのゲームログを足す。値の名前が使えないものならエラーにする。
    #[track_caller]
    pub fn write(&mut self, player_id: Ulid, log: GameLog) -> Result<(), Internal> {
        log.validate()?;
        self.entries.push((player_id, log));
        Ok(())
    }

    /// まだ1件も足していないか。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn with_optional_request_id(mut self, request_id: Option<String>) -> Self {
        self.request_id = request_id;
        self
    }

    fn validate(&self) -> Result<(), Internal> {
        let ascii = self.method.bytes().all(|b| b.is_ascii_graphic());
        if self.method.is_empty() || self.method.len() > MAX_METHOD_LEN || !ascii {
            return Err(Internal::msg(format!(
                "game log method {:?} must be 1..={MAX_METHOD_LEN} printable ASCII characters",
                self.method
            )));
        }
        Ok(())
    }
}

async fn write_to_database(tx: &mut Transaction<'_, MySql>, logs: &GameLogs) -> sqlx::Result<()> {
    for (player_id, log) in &logs.entries {
        sqlx::query(
            "INSERT INTO game_logs (player_id, request_id, method, kind, body) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(player_id.to_bytes().as_slice())
        .bind(logs.request_id.as_deref())
        .bind(&logs.method)
        .bind(&log.kind)
        .bind(log.body().to_string())
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

fn write_to_stdout(logs: &GameLogs) {
    for (player_id, log) in &logs.entries {
        let quantity = log.quantity;
        tracing::info!(
            target: GAME_LOG_TARGET,
            log_type = "game",
            player_id = %player_id,
            request_id = logs.request_id.as_deref(),
            "rpc.method" = logs.method,
            kind = log.kind,
            target_id = log.target_id.as_deref(),
            delta = quantity.map(|quantity| quantity.delta),
            after = quantity.map(|quantity| quantity.after),
            reason = log.reason,
            { GAME_LOG_FIELDS } = log.fields_json(),
            "game log"
        );
    }
}
