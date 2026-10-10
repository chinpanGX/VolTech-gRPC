/// ガチャを1回引くのに使うジェムの数。
pub const DRAW_COST: i64 = 300;

/// ガチャから出るアイテムと、出やすさ(重み)。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GachaItem {
    pub item_id: i32,
    pub weight: u32,
    /// レアか。レアが出たら、通知(`RareItemObtained`)を返す。
    pub rare: bool,
}

/// 排出物の表。サンプルなので、マスタにせずコードに持つ。
pub const ITEMS: &[GachaItem] = &[
    GachaItem {
        item_id: 1,
        weight: 70,
        rare: false,
    },
    GachaItem {
        item_id: 2,
        weight: 25,
        rare: false,
    },
    GachaItem {
        item_id: 3,
        weight: 5,
        rare: true,
    },
];

/// 重みの合計。`pick` に渡す値は `0..total_weight()` にする。
pub fn total_weight() -> u32 {
    ITEMS.iter().map(|item| item.weight).sum()
}

/// `0..total_weight()` の値から、重みに合わせてアイテムを選ぶ。
pub fn pick(roll: u32) -> GachaItem {
    let mut remaining = roll;
    for item in ITEMS {
        if remaining < item.weight {
            return *item;
        }
        remaining -= item.weight;
    }
    panic!("roll {roll} is out of 0..{}", total_weight());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 重みの境目で、選ばれるアイテムが切り替わる(0〜69 は1、70〜94 は2、95〜99 は3)。
    #[test]
    fn pick_follows_weights() {
        assert_eq!(pick(0).item_id, 1);
        assert_eq!(pick(69).item_id, 1);
        assert_eq!(pick(70).item_id, 2);
        assert_eq!(pick(94).item_id, 2);
        assert_eq!(pick(95).item_id, 3);
        assert_eq!(pick(total_weight() - 1).item_id, 3);
    }
}
