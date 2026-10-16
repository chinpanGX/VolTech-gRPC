use std::fmt;
use std::io::Write;

use serde_json::Value;
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::registry::LookupSpan;

use crate::config::FieldNames;

/// span と event の値を一番上の項目に並べて、1行1件の JSON で出す Layer。
///
/// tracing-subscriber の JSON は span の値を `span`・`spans` の下に入れ子にするので、
/// `request_id` などで検索・集計しやすいように自前で書く。
pub(crate) struct JsonLayer<W> {
    pub(crate) make_writer: W,
    pub(crate) field_names: FieldNames,
    pub(crate) service_name: String,
    pub(crate) environment: String,
}

/// span が持つ値。作ったときと `Span::record` のときに更新する。
struct SpanFields(Vec<(&'static str, Value)>);

impl<S, W> Layer<S> for JsonLayer<W>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    W: for<'w> MakeWriter<'w> + 'static,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else {
            return;
        };
        let mut fields = SpanFields(Vec::new());
        attrs.record(&mut FieldVisitor(&mut fields.0));
        span.extensions_mut().insert(fields);
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else {
            return;
        };
        if let Some(fields) = span.extensions_mut().get_mut::<SpanFields>() {
            values.record(&mut FieldVisitor(&mut fields.0));
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let mut event_fields = Vec::new();
        event.record(&mut FieldVisitor(&mut event_fields));
        let message = take(&mut event_fields, "message").unwrap_or(Value::String(String::new()));

        let metadata = event.metadata();
        let (time_key, level_key, level) = match self.field_names {
            FieldNames::Standard => ("timestamp", "level", standard_level(metadata.level())),
            FieldNames::Gcp => ("time", "severity", gcp_severity(metadata.level())),
        };
        let mut line = Line(vec![
            (time_key, Value::String(now())),
            (level_key, Value::String(level.to_owned())),
            ("message", message),
            ("target", Value::String(metadata.target().to_owned())),
            ("service.name", Value::String(self.service_name.clone())),
            (
                "deployment.environment.name",
                Value::String(self.environment.clone()),
            ),
        ]);
        let reserved = line.0.len();

        // 外側の span から順に足し、同じ名前は内側の span・event の値で上書きする
        if let Some(scope) = ctx.event_scope(event) {
            for span in scope.from_root() {
                if let Some(fields) = span.extensions().get::<SpanFields>() {
                    for (name, value) in &fields.0 {
                        line.set(reserved, name, value.clone());
                    }
                }
            }
        }
        for (name, value) in event_fields {
            line.set(reserved, name, value);
        }

        let mut buf = line.to_json();
        buf.push(b'\n');
        // 1行を1回の write_all で書き、ほかのスレッドの行と混ざらないようにする
        let _ = self.make_writer.make_writer_for(metadata).write_all(&buf);
    }
}

struct Line(Vec<(&'static str, Value)>);

impl Line {
    /// 値を足す。基盤が付ける項目(先頭の `reserved` 件)は上書きしない。
    fn set(&mut self, reserved: usize, name: &'static str, value: Value) {
        match self.0.iter().position(|(key, _)| *key == name) {
            Some(index) if index < reserved => {}
            Some(index) => self.0[index].1 = value,
            None => self.0.push((name, value)),
        }
    }

    fn to_json(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(256);
        buf.push(b'{');
        for (index, (key, value)) in self.0.iter().enumerate() {
            if index > 0 {
                buf.push(b',');
            }
            // 文字列と serde_json::Value の書き出しは失敗しない
            let _ = serde_json::to_writer(&mut buf, key);
            buf.push(b':');
            let _ = serde_json::to_writer(&mut buf, value);
        }
        buf.push(b'}');
        buf
    }
}

fn take(fields: &mut Vec<(&'static str, Value)>, name: &str) -> Option<Value> {
    let index = fields.iter().position(|(key, _)| *key == name)?;
    Some(fields.remove(index).1)
}

/// 今の時刻を、UTC の RFC 3339(ミリ秒まで)で返す。
pub(crate) fn now() -> String {
    format!("{:.3}", jiff::Timestamp::now())
}

fn standard_level(level: &Level) -> &'static str {
    match *level {
        Level::TRACE => "TRACE",
        Level::DEBUG => "DEBUG",
        Level::INFO => "INFO",
        Level::WARN => "WARN",
        Level::ERROR => "ERROR",
    }
}

// Cloud Logging の LogSeverity の名前に合わせる。TRACE に当たるものは無いので DEBUG にする
fn gcp_severity(level: &Level) -> &'static str {
    match *level {
        Level::TRACE | Level::DEBUG => "DEBUG",
        Level::INFO => "INFO",
        Level::WARN => "WARNING",
        Level::ERROR => "ERROR",
    }
}

struct FieldVisitor<'a>(&'a mut Vec<(&'static str, Value)>);

impl FieldVisitor<'_> {
    fn set(&mut self, field: &Field, value: Value) {
        let name = field.name();
        match self.0.iter_mut().find(|(key, _)| *key == name) {
            Some(entry) => entry.1 = value,
            None => self.0.push((name, value)),
        }
    }
}

impl Visit for FieldVisitor<'_> {
    fn record_f64(&mut self, field: &Field, value: f64) {
        self.set(field, Value::from(value));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.set(field, Value::from(value));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.set(field, Value::from(value));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.set(field, Value::from(value));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.set(field, Value::from(value));
    }

    fn record_error(&mut self, field: &Field, value: &(dyn std::error::Error + 'static)) {
        self.set(field, Value::from(value.to_string()));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.set(field, Value::from(format!("{value:?}")));
    }
}
