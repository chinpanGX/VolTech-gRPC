use crate::badge::BadgeCollector;
use crate::diff::PlayerDiffCollector;
use crate::notification::NotificationCollector;

/// 共通レスポンス項目のコレクターをまとめたもの。1つのリクエストで1つ作り、`service/` の関数に渡す。
///
/// ハンドラーは、処理が成功したら中身をレスポンスの `player_diff`・`badges`・`notifications` に詰める。
/// トランザクションがロールバックされたら、レスポンスごと捨てられるので、中身も残らない。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Collectors {
    pub diff: PlayerDiffCollector,
    pub badges: BadgeCollector,
    pub notifications: NotificationCollector,
}
