use std::fmt::{self, Display};
use std::str::FromStr;

/// アクセスログから除く RPC の既定。ヘルスチェックとリフレクションは数秒おきに呼ばれてログを埋めるので除く。
pub const DEFAULT_EXCLUDED_RPCS: &[&str] = &[
    "grpc.health.v1.Health",
    "grpc.reflection.v1.ServerReflection",
    "grpc.reflection.v1alpha.ServerReflection",
];

/// ログの設定。基盤は環境変数を読まないので、アプリが読んで渡す。
///
/// `LogConfig::new` で既定の値を入れてから、変えたい項目だけを書き換える。
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct LogConfig {
    /// 出力の形式。既定は JSON。
    pub format: LogFormat,
    /// `RUST_LOG` の形式のフィルター。既定は `info`。
    pub filter: String,
    /// JSON の項目名の付け方。既定は標準。
    pub field_names: FieldNames,
    /// どの行にも付けるサービス名(`service.name`)。
    pub service_name: String,
    /// どの行にも付ける環境名(`deployment.environment.name`)。
    pub environment: String,
    /// アクセスログから除く RPC。サービス名(例: `grpc.health.v1.Health`)か、
    /// メソッドまで含めた名前(例: `app.v1.GachaService/Draw`)で書く。
    pub excluded_rpcs: Vec<String>,
}

impl LogConfig {
    /// サービス名と環境名を受け取り、ほかの項目を既定の値にして作る。
    pub fn new(service_name: impl Into<String>, environment: impl Into<String>) -> Self {
        Self {
            format: LogFormat::Json,
            filter: "info".to_owned(),
            field_names: FieldNames::Standard,
            service_name: service_name.into(),
            environment: environment.into(),
            excluded_rpcs: DEFAULT_EXCLUDED_RPCS
                .iter()
                .map(|&rpc| rpc.to_owned())
                .collect(),
        }
    }
}

/// ログの出力の形式。どちらも標準出力に出す。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogFormat {
    /// 1行1件の JSON(`json`)。
    Json,
    /// 人が読むためのテキスト(`text`)。出力先がターミナルで、`NO_COLOR` が無いときだけ色を付ける。
    Text,
}

impl FromStr for LogFormat {
    type Err = UnknownValueError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "json" => Ok(Self::Json),
            "text" => Ok(Self::Text),
            _ => Err(UnknownValueError::new(
                "log format",
                value,
                &["json", "text"],
            )),
        }
    }
}

/// JSON の項目名の付け方。送り先に合わせて選ぶ。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldNames {
    /// `timestamp`・`level`(`TRACE`〜`ERROR`)(`standard`)。
    Standard,
    /// Google Cloud Logging 向け。`time`・`severity`(`DEBUG`・`INFO`・`WARNING`・`ERROR`)(`gcp`)。
    Gcp,
}

impl FromStr for FieldNames {
    type Err = UnknownValueError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "standard" => Ok(Self::Standard),
            "gcp" => Ok(Self::Gcp),
            _ => Err(UnknownValueError::new(
                "log field names",
                value,
                &["standard", "gcp"],
            )),
        }
    }
}

/// 設定の値が、知っている値のどれでもなかった。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownValueError {
    kind: &'static str,
    value: String,
    expected: &'static [&'static str],
}

impl UnknownValueError {
    fn new(kind: &'static str, value: &str, expected: &'static [&'static str]) -> Self {
        Self {
            kind,
            value: value.to_owned(),
            expected,
        }
    }
}

impl Display for UnknownValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown {} {:?} (expected one of: {})",
            self.kind,
            self.value,
            self.expected.join(", ")
        )
    }
}

impl std::error::Error for UnknownValueError {}

#[cfg(test)]
mod tests {
    use super::*;

    /// 知っている名前だけを読み、書き間違いはエラーにする。
    #[test]
    fn parse_known_names_only() {
        assert_eq!("json".parse(), Ok(LogFormat::Json));
        assert_eq!("text".parse(), Ok(LogFormat::Text));
        assert_eq!("standard".parse(), Ok(FieldNames::Standard));
        assert_eq!("gcp".parse(), Ok(FieldNames::Gcp));

        let err = "jsn".parse::<LogFormat>().unwrap_err();
        assert_eq!(
            err.to_string(),
            r#"unknown log format "jsn" (expected one of: json, text)"#
        );
        assert!("JSON".parse::<LogFormat>().is_err());
        assert!("google".parse::<FieldNames>().is_err());
    }
}
