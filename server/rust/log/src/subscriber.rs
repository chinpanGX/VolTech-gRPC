use std::fmt::{self, Display};
use std::io::IsTerminal;

use tracing::Subscriber;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::{FilterExt, ParseError, filter_fn};
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::FormatTime;
use tracing_subscriber::layer::{Layer, SubscriberExt};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::{SubscriberInitExt, TryInitError};

use crate::config::{LogConfig, LogFormat};
use crate::json::{self, JsonLayer};
use crate::{GAME_LOG_TARGET, RPC_SPAN_TARGET};

/// 設定どおりにログを標準出力へ出す準備をし、プロセス全体のログの出力先にする。起動時に1回だけ呼ぶ。
///
/// フィルターの書き方が間違っているとき、すでに準備してあるときはエラーを返す。
/// ほかの Layer(OpenTelemetry など)と組み合わせるときは、代わりに `layer` を使う。
pub fn init(config: &LogConfig) -> Result<(), InitError> {
    tracing_subscriber::registry()
        .with(layer(config)?)
        .try_init()
        .map_err(InitError::AlreadyInitialized)
}

/// 設定どおりにログを標準出力へ出す Layer を作る。`tracing_subscriber::registry().with(...)` に足して使う。
///
/// フィルターの書き方が間違っているときはエラーを返す。
pub fn layer<S>(config: &LogConfig) -> Result<Box<dyn Layer<S> + Send + Sync>, InitError>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    // NO_COLOR は設定ではなく、ターミナルの利用者が色を切るための慣習(https://no-color.org/)なので、ここで読む
    let ansi = std::io::stdout().is_terminal()
        && std::env::var_os("NO_COLOR").is_none_or(|value| value.is_empty());
    build(config, std::io::stdout, ansi)
}

/// `layer` と同じ Layer を、標準出力ではなく `make_writer` へ出すように作る(テストで出力を読むときなど)。テキストに色は付けない。
pub fn layer_with_writer<S, W>(
    config: &LogConfig,
    make_writer: W,
) -> Result<Box<dyn Layer<S> + Send + Sync>, InitError>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    W: for<'w> MakeWriter<'w> + Send + Sync + 'static,
{
    build(config, make_writer, false)
}

fn build<S, W>(
    config: &LogConfig,
    make_writer: W,
    ansi: bool,
) -> Result<Box<dyn Layer<S> + Send + Sync>, InitError>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    W: for<'w> MakeWriter<'w> + Send + Sync + 'static,
{
    let env_filter = EnvFilter::builder()
        .parse(&config.filter)
        .map_err(InitError::InvalidFilter)?;
    // RPC ごとの span は、フィルターのレベルを上げても有効にする。
    // 無効になると、その中の warn・error のログに request_id・player_id が付かなくなるため。
    // ゲームログは問い合わせ・補填に使う履歴なので、運用ログのフィルターで消さない
    let filter = env_filter.or(filter_fn(|metadata| {
        (metadata.is_span() && metadata.target() == RPC_SPAN_TARGET)
            || (metadata.is_event() && metadata.target() == GAME_LOG_TARGET)
    }));

    let layer = match config.format {
        LogFormat::Json => JsonLayer {
            make_writer,
            field_names: config.field_names,
            service_name: config.service_name.clone(),
            environment: config.environment.clone(),
        }
        .with_filter(filter)
        .boxed(),
        LogFormat::Text => tracing_subscriber::fmt::layer()
            .with_timer(UtcMillis)
            .with_ansi(ansi)
            .with_writer(make_writer)
            .with_filter(filter)
            .boxed(),
    };
    Ok(layer)
}

/// テキストの時刻を、JSON と同じ UTC の RFC 3339(ミリ秒まで)にする。
struct UtcMillis;

impl FormatTime for UtcMillis {
    fn format_time(&self, w: &mut Writer<'_>) -> fmt::Result {
        w.write_str(&json::now())
    }
}

/// ログの準備に失敗した。
#[derive(Debug)]
pub enum InitError {
    /// フィルター(`RUST_LOG` の形式)の書き方が間違っている。
    InvalidFilter(ParseError),
    /// プロセス全体のログの出力先が、すでに準備してある。
    AlreadyInitialized(TryInitError),
}

impl Display for InitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFilter(err) => write!(f, "invalid log filter: {err}"),
            Self::AlreadyInitialized(err) => write!(f, "logging is already initialized: {err}"),
        }
    }
}

impl std::error::Error for InitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidFilter(err) => Some(err),
            Self::AlreadyInitialized(err) => Some(err),
        }
    }
}
