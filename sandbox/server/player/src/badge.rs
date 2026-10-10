use std::collections::BTreeSet;

/// サーバーが計算するバッジの種類(`sandbox/v1/badge.proto` の `BadgeType`)。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BadgeKind {
    /// 今日の無料ガチャをまだ引いていない。
    FreeGacha,
    /// カウンターが 0 より大きい(件数付き)。
    Counter,
}

impl BadgeKind {
    /// すべての種類。サインインで全種類を返すときに使う。
    pub const ALL: &[BadgeKind] = &[BadgeKind::FreeGacha, BadgeKind::Counter];
}

/// 1つのリクエストの間に、状態が変わったかもしれないバッジの種類を集める(バッジのコレクター)。
///
/// 書き込み関数が、影響するバッジの種類を `mark` で記録する(例: 無料ガチャを引く関数は `FreeGacha`)。
/// 状態はここでは持たず、レスポンスを作るときに、記録された種類だけを今のデータから計算する。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BadgeCollector {
    all: bool,
    marked: BTreeSet<BadgeKind>,
}

impl BadgeCollector {
    /// `kind` のバッジの状態が変わったかもしれないことを記録する。
    pub fn mark(&mut self, kind: BadgeKind) {
        self.marked.insert(kind);
    }

    /// すべての種類を記録する。サインインとホームで使う(運営の操作で変わるバッジも、ここで更新される)。
    pub fn mark_all(&mut self) {
        self.all = true;
    }

    /// 記録された種類を、決まった順に返す。
    pub fn kinds(&self) -> Vec<BadgeKind> {
        if self.all {
            BadgeKind::ALL.to_vec()
        } else {
            self.marked.iter().copied().collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 何も記録しなければ空。同じ種類を何回記録しても1つ。
    #[test]
    fn mark_deduplicates() {
        let mut badges = BadgeCollector::default();
        assert!(badges.kinds().is_empty());

        badges.mark(BadgeKind::FreeGacha);
        badges.mark(BadgeKind::FreeGacha);
        assert_eq!(badges.kinds(), vec![BadgeKind::FreeGacha]);
    }

    /// mark_all なら、記録していない種類も含めて全種類を返す。
    #[test]
    fn mark_all_returns_every_kind() {
        let mut badges = BadgeCollector::default();
        badges.mark_all();
        assert_eq!(badges.kinds(), BadgeKind::ALL.to_vec());
    }
}
