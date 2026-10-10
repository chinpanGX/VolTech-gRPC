use ulid::Ulid;

/// プレイヤー。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Player {
    pub player_id: Ulid,
    pub gems: i64,
    /// 基盤の冪等性を確かめるためのカウンター(`SandboxService/Increment` で増える)。
    pub counter: i64,
    /// BAN・アカウント停止中か。
    pub suspended: bool,
}

/// 所持しているアイテム。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerItem {
    pub item_id: i32,
    pub count: i64,
}
