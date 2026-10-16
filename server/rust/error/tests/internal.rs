use std::error::Error;
use std::fmt::{self, Display};
use std::io;
use std::sync::{Arc, Mutex};

use tonic::{Code, Status};
use tonic_types::StatusExt;
use tracing_subscriber::fmt::MakeWriter;
use voltech_grpc_error::{Internal, ResultExt, failed_precondition, internal, reserved};

/// ログの出力をためておく書き込み先。
#[derive(Clone, Default)]
struct Output(Arc<Mutex<Vec<u8>>>);

impl Output {
    fn text(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}

impl io::Write for Output {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for Output {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

/// `f` の中で出たログを、テキストで返す。
fn capture_logs<T>(f: impl FnOnce() -> T) -> (T, String) {
    let output = Output::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(output.clone())
        .with_ansi(false)
        .finish();
    let value = tracing::subscriber::with_default(subscriber, f);
    (value, output.text())
}

/// 原因(`source()`)を持つエラー。
#[derive(Debug)]
struct QueryFailed(io::Error);

impl Display for QueryFailed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("query failed")
    }
}

impl Error for QueryFailed {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}

fn query() -> Result<(), QueryFailed> {
    Err(QueryFailed(io::Error::other("connection reset")))
}

/// `?` で作った `Internal` を `Status` にすると、INTERNAL(INTERNAL_ERROR)になり、説明・原因の連なり・`?` の場所を error で出す。
#[test]
fn internal_logs_whole_cause_chain_with_location() {
    let line = line!() + 2;
    let grant = || -> Result<(), Internal> {
        query().context("grant item")?;
        Ok(())
    };

    let (status, logs) = capture_logs(|| Status::from(grant().unwrap_err()));

    assert_eq!(status.code(), Code::Internal);
    assert_eq!(status.message(), "internal error");
    let info = status.get_details_error_info().unwrap();
    assert_eq!(info.reason, reserved::INTERNAL_ERROR.reason());
    assert!(logs.contains("ERROR"), "{logs}");
    assert!(
        logs.contains("grant item: query failed: connection reset"),
        "{logs}"
    );
    assert!(logs.contains(&format!("code.line.number={line}")), "{logs}");
    // クライアントには原因を返さない
    assert!(!status.message().contains("connection reset"));
}

/// 業務エラーの `Status` に `?` を使って `Internal` に包んでも、`Status` に戻すと元の業務エラーのままで、error のログも出ない。
#[test]
fn business_error_through_internal_stays_as_is() {
    let check = || -> Result<(), Internal> {
        Err(failed_precondition(
            reserved::DATE_CHANGED,
            "game date changed",
        ))?;
        Ok(())
    };

    let (status, logs) = capture_logs(|| internal(check().unwrap_err()));

    assert_eq!(status.code(), Code::FailedPrecondition);
    let info = status.get_details_error_info().unwrap();
    assert_eq!(info.reason, reserved::DATE_CHANGED.reason());
    assert!(logs.is_empty(), "{logs}");
}

/// `or_internal` は、呼んだ場所を発生した場所として出す(`.map_err(internal)` では場所が残らない)。
#[test]
fn or_internal_records_call_site() {
    let (status, logs) = capture_logs(|| query().or_internal().unwrap_err());
    let line = line!() - 1;

    assert_eq!(status.code(), Code::Internal);
    assert!(logs.contains("query failed: connection reset"), "{logs}");
    assert!(logs.contains(&format!("code.line.number={line}")), "{logs}");
    assert!(logs.contains("internal.rs"), "{logs}");
}

/// 元のエラーが無いときは、`Internal::msg` の説明だけを出す。
#[test]
fn message_only_internal() {
    let error = Internal::msg("RpcMethod is missing");
    assert_eq!(error.to_string(), "RpcMethod is missing");

    let boxed: Box<dyn Error + Send + Sync> = "player_id must be 16 bytes".into();
    let error = Internal::from_boxed(boxed).context("resolve player");
    assert_eq!(
        error.to_string(),
        "resolve player: player_id must be 16 bytes"
    );
}
