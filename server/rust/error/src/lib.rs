pub mod code;

use std::collections::HashMap;
use std::fmt::Display;

use tonic::{Code, Status};
use tonic_types::{ErrorDetails, StatusExt};

const ERROR_DOMAIN: &str = "framework";

/// 業務エラー(`FAILED_PRECONDITION` + `ErrorInfo.reason`)を作る。
pub fn failed_precondition(reason: &str, message: &str) -> Status {
    with_reason(Code::FailedPrecondition, reason, message, HashMap::new())
}

/// 業務エラーを、表示用の値(`ErrorInfo.metadata`)付きで作る。キー `action` は action の上書きに使うので避ける。
pub fn failed_precondition_with_metadata(
    reason: &str,
    message: &str,
    metadata: HashMap<String, String>,
) -> Status {
    with_reason(Code::FailedPrecondition, reason, message, metadata)
}

/// サーバー内部エラーを作る。詳細はクライアントに返さず、ログにだけ出す。
pub fn internal(err: impl Display) -> Status {
    tracing::error!(%err, "internal error");
    with_reason(
        Code::Internal,
        code::INTERNAL_ERROR,
        "internal error",
        HashMap::new(),
    )
}

fn with_reason(
    code: Code,
    reason: &str,
    message: &str,
    metadata: HashMap<String, String>,
) -> Status {
    Status::with_error_details(
        code,
        message,
        ErrorDetails::with_error_info(reason, ERROR_DOMAIN, metadata),
    )
}
