use std::future::Future;

/// プレイヤーの ID。DB には BINARY(16)、API には 26 文字の文字列で持つ。
pub type PlayerId = ulid::Ulid;
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// プレイヤーの状態。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerStatus {
    /// `player_id` は発行済みだが、プレイヤーがまだ作られていない(サインアップの前)。
    NotCreated,
    Active,
    /// BAN・アカウント停止中。
    Suspended,
}

/// `player_id` からプレイヤーの状態を引く。`players` テーブルを持つアプリが実装する。
pub trait PlayerResolver: Clone + Send + Sync + 'static {
    /// `player_id` のプレイヤーの状態を返す。
    fn resolve(
        &self,
        player_id: PlayerId,
    ) -> impl Future<Output = Result<PlayerStatus, BoxError>> + Send;
}
