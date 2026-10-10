use sqlx::MySqlPool;
use voltech_grpc_auth::{BoxError, PlayerId, PlayerResolver, PlayerStatus};

/// `players` テーブルからプレイヤーを引く、`PlayerResolver` の実装。
#[derive(Clone)]
pub struct PlayerLookup {
    pool: MySqlPool,
}

impl PlayerLookup {
    /// `players` テーブルを読む DB を受け取って作る。
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

impl PlayerResolver for PlayerLookup {
    async fn resolve(&self, player_id: PlayerId) -> Result<PlayerStatus, BoxError> {
        let status = match player::service::find_by_id(&self.pool, player_id).await? {
            None => PlayerStatus::NotCreated,
            Some(player) if player.suspended => PlayerStatus::Suspended,
            Some(_) => PlayerStatus::Active,
        };
        Ok(status)
    }
}
