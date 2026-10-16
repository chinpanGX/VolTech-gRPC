//! サーバーの運用ログ。ログの出力の準備と、RPC ごとの span・アクセスログの Layer。

mod config;
mod json;
mod method;
mod rpc;
mod subscriber;

pub use config::{DEFAULT_EXCLUDED_RPCS, FieldNames, LogConfig, LogFormat, UnknownValueError};
pub use method::RpcMethod;
pub use rpc::{AccessLogBody, RpcLogLayer, RpcLogService};
pub use subscriber::{InitError, init, layer, layer_with_writer};

/// RPC ごとの span の target。レベルのフィルターにかかわらず有効にする。
pub const RPC_SPAN_TARGET: &str = "voltech_grpc_log::rpc";
/// アクセスログの target。
pub const ACCESS_LOG_TARGET: &str = "voltech_grpc_log::access";
/// 標準出力に出すゲームログの target。レベルのフィルターにかかわらず出す。
pub const GAME_LOG_TARGET: &str = "voltech_grpc_log::game";

/// RPC ごとの span の項目の名前。基盤のほかのクレートは、この名前で span に書き込む。
pub mod field {
    /// `x-request-id`。形が正しいときだけ入る。
    pub const REQUEST_ID: &str = "request_id";
    /// 認証したプレイヤー。`AuthLayer` が書き込む。
    pub const PLAYER_ID: &str = "player_id";
    /// 再送で、保存済みのレスポンスを返したか。`Transactions::run` が書き込む。
    pub const REPLAYED: &str = "replayed";
}

/// リクエストの ID を入れるメタデータ。
pub const REQUEST_ID_HEADER: &str = "x-request-id";
/// `x-request-id` の長さの上限。
pub const MAX_REQUEST_ID_LEN: usize = 64;

/// `x-request-id` の形(1..=64 文字の英数字と `-`)が正しいかを返す。
pub fn is_valid_request_id(request_id: &str) -> bool {
    !request_id.is_empty()
        && request_id.len() <= MAX_REQUEST_ID_LEN
        && request_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 英数字と `-` の 1..=64 文字だけを正しい形とする。
    #[test]
    fn request_id_must_be_short_alphanumeric() {
        assert!(is_valid_request_id("01J9Z3-abc"));
        assert!(is_valid_request_id(&"a".repeat(MAX_REQUEST_ID_LEN)));

        for request_id in ["", "a b", "a\nb", "あ", &"a".repeat(MAX_REQUEST_ID_LEN + 1)] {
            assert!(!is_valid_request_id(request_id), "{request_id:?}");
        }
    }
}
