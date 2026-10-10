use std::collections::BTreeMap;

use crate::model::PlayerItem;

/// 1つのリクエストの間に変わったプレイヤーのデータを集める(差分のコレクター)。
///
/// 書き込み関数が、DB に書くついでに変更後の値を記録する。同じものが何回変わっても、最後の値だけを持つ。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlayerDiffCollector {
    gems: Option<i64>,
    items: BTreeMap<i32, i64>,
    counter: Option<i64>,
}

impl PlayerDiffCollector {
    /// 変更後のジェムを記録する。
    pub fn record_gems(&mut self, gems: i64) {
        self.gems = Some(gems);
    }

    /// 変更後のアイテムの個数を記録する。0 個なら、無くなったものとして扱う。
    pub fn record_item(&mut self, item: PlayerItem) {
        self.items.insert(item.item_id, item.count);
    }

    /// 変更後のカウンターを記録する。
    pub fn record_counter(&mut self, counter: i64) {
        self.counter = Some(counter);
    }

    /// 変更後のジェム。変わっていなければ `None`。
    pub fn gems(&self) -> Option<i64> {
        self.gems
    }

    /// 変更後のカウンター。変わっていなければ `None`。
    pub fn counter(&self) -> Option<i64> {
        self.counter
    }

    /// 追加・更新されたアイテム(変更後の個数)を、item_id の順に返す。
    pub fn upserted_items(&self) -> impl Iterator<Item = PlayerItem> + '_ {
        self.items
            .iter()
            .filter(|(_, count)| **count > 0)
            .map(|(item_id, count)| PlayerItem {
                item_id: *item_id,
                count: *count,
            })
    }

    /// 無くなったアイテムの item_id を、順に返す。
    pub fn removed_items(&self) -> impl Iterator<Item = i32> + '_ {
        self.items
            .iter()
            .filter(|(_, count)| **count <= 0)
            .map(|(item_id, _)| *item_id)
    }

    /// アイテムが変わったか。
    pub fn has_item_changes(&self) -> bool {
        !self.items.is_empty()
    }

    /// 何も変わっていないか。
    pub fn is_empty(&self) -> bool {
        self.gems.is_none() && self.items.is_empty() && self.counter.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 同じアイテムが何回変わっても最後の個数だけを持ち、0 個になったものは無くなったものとして返す。
    #[test]
    fn keeps_last_values_and_splits_removed_items() {
        let mut diff = PlayerDiffCollector::default();
        assert!(diff.is_empty());

        diff.record_gems(100);
        diff.record_gems(50);
        diff.record_item(PlayerItem {
            item_id: 2,
            count: 1,
        });
        diff.record_item(PlayerItem {
            item_id: 2,
            count: 3,
        });
        diff.record_item(PlayerItem {
            item_id: 1,
            count: 0,
        });

        assert_eq!(diff.gems(), Some(50));
        assert_eq!(diff.counter(), None);
        assert_eq!(
            diff.upserted_items().collect::<Vec<_>>(),
            vec![PlayerItem {
                item_id: 2,
                count: 3
            }]
        );
        assert_eq!(diff.removed_items().collect::<Vec<_>>(), vec![1]);
    }

    /// カウンターだけが変わっても、空ではない。
    #[test]
    fn counter_only_is_not_empty() {
        let mut diff = PlayerDiffCollector::default();
        diff.record_counter(5);
        assert!(!diff.is_empty());
        assert_eq!(diff.counter(), Some(5));
    }
}
