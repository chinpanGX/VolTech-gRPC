mod common;

use common::{Output, config, subscriber};
use voltech_grpc_log::{
    FieldNames, GAME_LOG_FIELDS, GAME_LOG_TARGET, InitError, LogFormat, RPC_SPAN_TARGET,
};

/// JSON の1行には、時刻・レベル・メッセージ・サービス名・環境名と、span と event の値が一番上の項目として並ぶ。
#[test]
fn json_line_flattens_span_and_event_fields() {
    let output = Output::default();
    tracing::subscriber::with_default(subscriber(&config(), &output), || {
        let span = tracing::info_span!("outer", request_id = "req-1", shared = "outer");
        let _entered = span.enter();
        let inner = tracing::info_span!("inner", shared = "inner");
        let _inner = inner.enter();
        tracing::info!(gems = 300, "drew gacha");
    });

    let lines = output.lines();
    assert_eq!(lines.len(), 1);
    let line = &lines[0];
    assert_eq!(line["level"], "INFO");
    assert_eq!(line["message"], "drew gacha");
    assert_eq!(line["target"], "format");
    assert_eq!(line["service.name"], "test-server");
    assert_eq!(line["deployment.environment.name"], "test");
    assert_eq!(line["request_id"], "req-1");
    assert_eq!(line["shared"], "inner");
    assert_eq!(line["gems"], 300);
    assert!(line.get("span").is_none());

    // 例: 2026-10-16T01:02:03.456Z
    let timestamp = line["timestamp"].as_str().unwrap();
    assert_eq!(timestamp.len(), 24, "{timestamp}");
    assert_eq!(&timestamp[19..20], ".", "{timestamp}");
    assert!(timestamp.ends_with('Z'), "{timestamp}");
}

/// span に後から書き込んだ値も、その後のログに付く。
#[test]
fn recorded_span_fields_are_included() {
    let output = Output::default();
    tracing::subscriber::with_default(subscriber(&config(), &output), || {
        let span = tracing::info_span!("rpc", player_id = tracing::field::Empty);
        let _entered = span.enter();
        tracing::info!("before");
        span.record("player_id", "01J9Z3");
        tracing::info!("after");
    });

    let lines = output.lines();
    assert!(lines[0].get("player_id").is_none());
    assert_eq!(lines[1]["player_id"], "01J9Z3");
}

/// 基盤が付ける項目(レベルなど)は、同じ名前の値をログに書いても上書きされない。
#[test]
fn reserved_fields_are_not_overwritten() {
    let output = Output::default();
    tracing::subscriber::with_default(subscriber(&config(), &output), || {
        tracing::info!(level = "fake", target = "fake", "message");
    });

    let line = &output.lines()[0];
    assert_eq!(line["level"], "INFO");
    assert_eq!(line["target"], "format");
}

/// GCP 向けの項目名では、時刻を `time`、レベルを `severity` にし、warn を `WARNING` と書く。
#[test]
fn gcp_field_names_use_severity() {
    let mut config = config();
    config.field_names = FieldNames::Gcp;
    let output = Output::default();
    tracing::subscriber::with_default(subscriber(&config, &output), || {
        tracing::warn!("slow query");
    });

    let line = &output.lines()[0];
    assert_eq!(line["severity"], "WARNING");
    assert!(line["time"].is_string());
    assert!(line.get("level").is_none());
    assert!(line.get("timestamp").is_none());
}

/// フィルターを warn にしても RPC の span は有効で、warn のログに span の値が付き、info のログは出ない。
#[test]
fn rpc_span_survives_higher_filter_level() {
    let mut config = config();
    config.filter = "warn".to_owned();
    let output = Output::default();
    tracing::subscriber::with_default(subscriber(&config, &output), || {
        let span = tracing::info_span!(target: RPC_SPAN_TARGET, "rpc", request_id = "req-1");
        let _entered = span.enter();
        tracing::info!("hidden");
        tracing::warn!("shown");
    });

    let lines = output.lines();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["message"], "shown");
    assert_eq!(lines[0]["request_id"], "req-1");
}

/// フィルターでレベルを上げても・target を絞っても、ゲームログの target のログは出る。
#[test]
fn game_log_survives_filter() {
    let mut config = config();
    config.filter = "warn,voltech_grpc_log=off".to_owned();
    let output = Output::default();
    tracing::subscriber::with_default(subscriber(&config, &output), || {
        tracing::info!(target: GAME_LOG_TARGET, log_type = "game", "game log");
        tracing::info!("hidden");
    });

    let lines = output.lines();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["message"], "game log");
    assert_eq!(lines[0]["log_type"], "game");
}

/// ゲームログの名前付きの値(JSON の文字列)は、一番上の項目に展開する。基盤が付ける項目は上書きしない。
/// ゲームログでないログでは展開しない。
#[test]
fn game_log_fields_are_flattened() {
    let output = Output::default();
    let fields = r#"{"unit_id":"u-1","slots":[1,2],"level":"x"}"#;
    let other = r#"{"unit_id":"u-1"}"#;
    tracing::subscriber::with_default(subscriber(&config(), &output), || {
        // tracing のマクロは、定数の名前の項目を先頭に置けないので、ほかの項目の後に置く
        tracing::info!(target: GAME_LOG_TARGET, log_type = "game", { GAME_LOG_FIELDS } = fields, "game log");
        tracing::info!(log_type = "other", { GAME_LOG_FIELDS } = other, "other");
    });

    let lines = output.lines();
    assert_eq!(lines[0]["unit_id"], "u-1");
    assert_eq!(lines[0]["slots"], serde_json::json!([1, 2]));
    assert_eq!(lines[0]["level"], "INFO");
    assert!(lines[0].get(GAME_LOG_FIELDS).is_none());
    assert!(lines[1].get("unit_id").is_none());
    assert_eq!(lines[1][GAME_LOG_FIELDS], r#"{"unit_id":"u-1"}"#);
}

/// フィルターの書き方が間違っていると、Layer を作るときにエラーになる。
#[test]
fn invalid_filter_is_rejected() {
    let mut config = config();
    config.filter = "info,=warn=".to_owned();
    let result = voltech_grpc_log::layer_with_writer::<tracing_subscriber::Registry, _>(
        &config,
        Output::default(),
    );
    assert!(matches!(result, Err(InitError::InvalidFilter(_))));
}

/// テキストの形式では、時刻とメッセージと span の値を1行に出し、色は付けない。
#[test]
fn text_format_writes_plain_lines() {
    let mut config = config();
    config.format = LogFormat::Text;
    let output = Output::default();
    tracing::subscriber::with_default(subscriber(&config, &output), || {
        let span = tracing::info_span!("rpc", request_id = "req-1");
        let _entered = span.enter();
        tracing::info!("drew gacha");
    });

    let text = output.text();
    assert!(text.contains("drew gacha"), "{text}");
    assert!(text.contains("request_id=\"req-1\""), "{text}");
    assert!(text.contains('Z'), "{text}");
    assert!(!text.contains('\x1b'), "{text}");
}
