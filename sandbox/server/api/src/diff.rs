use player::diff::PlayerDiffCollector;
use proto::sandbox::v1::{ItemsDiff, PlayerDiff};

use crate::player::item_to_proto;

/// 集めた差分をレスポンスの `player_diff` にする。何も変わっていなければ `None`(フィールド無し)。
pub(crate) fn to_proto(diff: &PlayerDiffCollector) -> Option<PlayerDiff> {
    if diff.is_empty() {
        return None;
    }

    Some(PlayerDiff {
        gems: diff.gems(),
        items: diff.has_item_changes().then(|| ItemsDiff {
            upserted: diff.upserted_items().map(item_to_proto).collect(),
            removed: diff.removed_items().collect(),
        }),
        counter: diff.counter(),
    })
}
