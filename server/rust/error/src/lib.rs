mod internal;

use std::collections::HashMap;
use std::fmt::{self, Display};

use tonic::{Code, Status};
use tonic_types::{ErrorDetails, StatusExt};

pub use internal::{Internal, ResultExt};

/// 基盤の予約エラー。`errors.toml` から生成する。
pub mod reserved {
    include!(concat!(env!("OUT_DIR"), "/errors.rs"));
}

/// エラーコード。`ErrorInfo` の domain と reason の組で見分ける。
///
/// 定数はエラーの定義ファイル(errors.toml)から `voltech-grpc-error-build` で生成する。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ErrorCode {
    domain: &'static str,
    reason: &'static str,
}

impl ErrorCode {
    /// domain と reason からエラーコードを作る。
    pub const fn new(domain: &'static str, reason: &'static str) -> Self {
        Self { domain, reason }
    }

    /// `ErrorInfo.domain` に入れる値。
    pub const fn domain(&self) -> &'static str {
        self.domain
    }

    /// `ErrorInfo.reason` に入れる値。
    pub const fn reason(&self) -> &'static str {
        self.reason
    }
}

impl Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.domain, self.reason)
    }
}

/// 業務エラー(`FAILED_PRECONDITION` + `ErrorInfo`)を作る。
pub fn failed_precondition(code: ErrorCode, message: &str) -> Status {
    with_code(Code::FailedPrecondition, code, message, HashMap::new())
}

/// 業務エラーを、表示用の値(`ErrorInfo.metadata`)付きで作る。キー `action` は action の上書きに使うので避ける。
pub fn failed_precondition_with_metadata(
    code: ErrorCode,
    message: &str,
    metadata: HashMap<String, String>,
) -> Status {
    with_code(Code::FailedPrecondition, code, message, metadata)
}

/// サーバー内部エラーを作る。原因をたどってログに error で出し、クライアントには詳細を返さない。
///
/// `std::error::Error` を実装したエラーと `Internal` を受け取る。文字列だけのときは `Internal::msg` で包む。
/// 受け取ったのが `tonic::Status` なら、INTERNAL にせずにそのまま返す。
/// `.map_err(internal)` と関数を渡すと発生した場所が残らないので、`ResultExt::or_internal` を使う。
#[track_caller]
pub fn internal(err: impl Into<Internal>) -> Status {
    err.into().into_status()
}

fn with_code(
    status: Code,
    code: ErrorCode,
    message: &str,
    metadata: HashMap<String, String>,
) -> Status {
    Status::with_error_details(
        status,
        message,
        ErrorDetails::with_error_info(code.reason, code.domain, metadata),
    )
}
