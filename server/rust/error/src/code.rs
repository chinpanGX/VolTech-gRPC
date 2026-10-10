//! 基盤が返す予約エラーコードの名前(`ErrorInfo.reason` に入れる値)。
//!
//! `ErrorCode` の enum はアプリ側で生成されるので、ここでは名前を文字列で持つ。
//! アプリの `error_code.proto` の値の名前と一致させること。

pub const INTERNAL_ERROR: &str = "ERROR_CODE_INTERNAL_ERROR";
pub const MAINTENANCE: &str = "ERROR_CODE_MAINTENANCE";
pub const UPDATE_REQUIRED: &str = "ERROR_CODE_UPDATE_REQUIRED";
pub const ACCOUNT_SUSPENDED: &str = "ERROR_CODE_ACCOUNT_SUSPENDED";
pub const DATE_CHANGED: &str = "ERROR_CODE_DATE_CHANGED";

/// 予約エラーコードの名前の一覧。アプリの `error_code.proto` との突き合わせに使う。
pub const ALL: &[&str] = &[
    INTERNAL_ERROR,
    MAINTENANCE,
    UPDATE_REQUIRED,
    ACCOUNT_SUSPENDED,
    DATE_CHANGED,
];
