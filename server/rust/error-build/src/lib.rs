//! エラーの定義ファイル(errors.toml)から、エラーコードの定数を生成する。build.rs から使う。
//!
//! ```no_run
//! // build.rs
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     voltech_grpc_error_build::compile("../../errors.toml")
//! }
//! ```
//!
//! 生成したコードは `include!(concat!(env!("OUT_DIR"), "/errors.rs"))` で取り込む。

use std::collections::HashSet;
use std::env;
use std::error::Error;
use std::fmt::{self, Display, Write};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// 基盤の予約エラーの domain。アプリの定義ファイルでは使えない。
pub const RESERVED_DOMAIN: &str = "voltech";

const OUTPUT_FILE: &str = "errors.rs";
// AIP-193 の ErrorInfo.reason の上限
const MAX_REASON_LEN: usize = 63;
// 生成するコードが自分で持つ定数の名前。reason の定数とぶつけない
const MEMBER_NAMES: &[&str] = &["DOMAIN", "ALL"];

/// 定義ファイルの中身が規則に合わない。
#[derive(Debug)]
pub struct InvalidErrors(String);

impl Display for InvalidErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for InvalidErrors {}

/// 定義ファイルからエラーコードの定数を生成し、`OUT_DIR/errors.rs` に書く。
pub fn compile(path: impl AsRef<Path>) -> Result<(), Box<dyn Error>> {
    configure().compile(path)
}

/// 生成の設定を作る。
pub fn configure() -> Builder {
    Builder { reserved: false }
}

/// 生成の設定。
pub struct Builder {
    reserved: bool,
}

impl Builder {
    /// 基盤の予約エラーを生成する。基盤の `voltech-grpc-error` だけが使う。
    #[doc(hidden)]
    pub fn reserved(mut self) -> Self {
        self.reserved = true;
        self
    }

    /// 定義ファイルからエラーコードの定数を生成し、`OUT_DIR/errors.rs` に書く。
    pub fn compile(self, path: impl AsRef<Path>) -> Result<(), Box<dyn Error>> {
        let path = path.as_ref();
        let source = fs::read_to_string(path)
            .map_err(|err| format!("{} を読めない: {err}", path.display()))?;
        let code = self
            .generate(&source)
            .map_err(|err| format!("{}: {err}", path.display()))?;

        let out_dir = PathBuf::from(env::var("OUT_DIR")?);
        fs::write(out_dir.join(OUTPUT_FILE), code)?;
        println!("cargo:rerun-if-changed={}", path.display());
        Ok(())
    }

    /// 定義ファイルの中身から、Rust のコードを作る。
    pub fn generate(&self, source: &str) -> Result<String, InvalidErrors> {
        let file: ErrorsFile =
            toml::from_str(source).map_err(|err| InvalidErrors(err.to_string()))?;
        self.validate(&file)?;

        // 基盤の中では crate::ErrorCode、アプリからは voltech_grpc_error::ErrorCode を指す
        let error_code = if self.reserved {
            "crate::ErrorCode"
        } else {
            "::voltech_grpc_error::ErrorCode"
        };
        let server_errors: Vec<&Entry> = file.errors.iter().filter(|e| !e.client_only).collect();

        let mut out = String::new();
        out.push_str(
            "// errors.toml から voltech-grpc-error-build が生成した。手で編集しない。\n\n",
        );
        out.push_str("/// エラーの domain(`ErrorInfo.domain`)。\n");
        writeln!(out, "pub const DOMAIN: &str = \"{}\";", file.domain).unwrap();
        for entry in &server_errors {
            out.push('\n');
            for line in entry.description.lines() {
                writeln!(out, "/// {line}").unwrap();
            }
            writeln!(
                out,
                "pub const {}: {error_code} = {error_code}::new(DOMAIN, \"{}\");",
                entry.reason, entry.reason
            )
            .unwrap();
        }
        out.push_str(
            "\n/// サーバーが返すエラーコードの一覧(クライアントだけが使うものは含めない)。\n",
        );
        write!(out, "pub const ALL: &[{error_code}] = &[").unwrap();
        for entry in &server_errors {
            write!(out, "{}, ", entry.reason).unwrap();
        }
        out.push_str("];\n");
        Ok(out)
    }

    fn validate(&self, file: &ErrorsFile) -> Result<(), InvalidErrors> {
        if !is_valid_domain(&file.domain) {
            return Err(InvalidErrors(format!(
                "domain は小文字・数字・ハイフンを . でつないだ名前にする(例: my-app): {}",
                file.domain
            )));
        }
        if self.reserved != (file.domain == RESERVED_DOMAIN) {
            return Err(InvalidErrors(if self.reserved {
                format!("基盤の予約エラーの domain は {RESERVED_DOMAIN} にする")
            } else {
                format!("domain {RESERVED_DOMAIN} は基盤の予約エラーが使うので、アプリでは使えない")
            }));
        }

        let mut seen = HashSet::new();
        for entry in &file.errors {
            if !is_valid_reason(&entry.reason) {
                return Err(InvalidErrors(format!(
                    "reason は大文字のスネークケース、{MAX_REASON_LEN}文字以内にする: {}",
                    entry.reason
                )));
            }
            if MEMBER_NAMES.contains(&entry.reason.as_str()) {
                return Err(InvalidErrors(format!(
                    "reason {} は、生成するコードの定数とぶつかるので使えない",
                    entry.reason
                )));
            }
            if !seen.insert(entry.reason.as_str()) {
                return Err(InvalidErrors(format!(
                    "reason が重複している: {}",
                    entry.reason
                )));
            }
            if entry.description.trim().is_empty() {
                return Err(InvalidErrors(format!(
                    "{}: description に用途を書く",
                    entry.reason
                )));
            }
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ErrorsFile {
    domain: String,
    #[serde(default)]
    errors: Vec<Entry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    reason: String,
    // サーバーは既定の action を使わないが、書き間違いはここで弾く
    #[serde(rename = "action")]
    _action: Action,
    #[serde(default)]
    client_only: bool,
    description: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Action {
    Stay,
    Retry,
    Home,
    Title,
    Update,
}

// 例: my-app、example.com
fn is_valid_domain(domain: &str) -> bool {
    !domain.is_empty()
        && domain.split('.').all(|label| {
            label.starts_with(|c: char| c.is_ascii_lowercase())
                && label
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        })
}

// AIP-193: [A-Z][A-Z0-9_]+[A-Z0-9]
fn is_valid_reason(reason: &str) -> bool {
    reason.len() >= 2
        && reason.len() <= MAX_REASON_LEN
        && reason.starts_with(|c: char| c.is_ascii_uppercase())
        && !reason.ends_with('_')
        && reason
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}
