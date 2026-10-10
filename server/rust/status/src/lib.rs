//! サーバーの状態(メンテナンス・強制アップデート・日付変更)の判定。
//!
//! 設定は DB の外(`server_status.toml` など)に置き、数秒おきに読み直す。

mod config;
mod layer;
mod source;
mod status;

pub use config::{GameDay, Maintenance, MinVersion, Platform, StatusConfig};
pub use layer::{DateChangeLayer, DateChangeService, ServerStatusLayer, ServerStatusService};
pub use source::{FileStatusSource, StatusSource};
pub use status::{Clock, ServerStatus, SystemClock};

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// リクエストを受けたときのゲーム内の日付。`ServerStatusLayer` がリクエストの extensions に入れる。
///
/// ハンドラーは日付で決まる処理(1日1回の報酬など)にこれを使う。レスポンスヘッダー `x-game-date` も同じ日付になる。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameDate(pub jiff::civil::Date);

/// リクエスト: アプリのプラットフォーム(`ios`・`android`・`windows`・`editor`)。
pub const APP_PLATFORM_HEADER: &str = "x-app-platform";
/// リクエスト: アプリのバージョン。表示用の「1.2.0」ではなく、リリースのたびに増やす整数。
pub const APP_VERSION_HEADER: &str = "x-app-version";
/// リクエスト: セッションの最初に受け取ったゲーム内の日付。レスポンス: 今のゲーム内の日付。形は `YYYY-MM-DD`。
pub const GAME_DATE_HEADER: &str = "x-game-date";
/// レスポンス: サーバー時刻(Unix 時間のミリ秒)。
pub const SERVER_TIME_HEADER: &str = "x-server-time";
/// `MAINTENANCE` の `ErrorInfo.metadata` のキー: 終了予定(RFC 3339、UTC)。
pub const MAINTENANCE_END_AT_KEY: &str = "end_at";
