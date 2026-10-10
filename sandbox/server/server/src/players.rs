use sqlx::{MySql, MySqlExecutor, MySqlPool, Transaction};
use ulid::Ulid;
use voltech_grpc_auth::{BoxError, PlayerId, PlayerResolver, PlayerStatus};

/// `sandbox_players` テーブルからプレイヤーを引く、`PlayerResolver` の実装。
#[derive(Clone)]
pub struct Players {
    pool: MySqlPool,
}

impl Players {
    /// `sandbox_players` テーブルを読む DB を受け取って作る。
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

impl PlayerResolver for Players {
    async fn resolve(&self, player_id: PlayerId) -> Result<PlayerStatus, BoxError> {
        let suspended: Option<i64> = sqlx::query_scalar(
            "SELECT suspended_at IS NOT NULL FROM sandbox_players WHERE player_id = ?",
        )
        .bind(player_id.to_bytes().as_slice())
        .fetch_optional(&self.pool)
        .await?;
        let status = match suspended {
            None => PlayerStatus::NotCreated,
            Some(0) => PlayerStatus::Active,
            Some(_) => PlayerStatus::Suspended,
        };
        Ok(status)
    }
}

/// プレイヤーを作る。作成済みなら何もしない。
pub(crate) async fn create(tx: &mut Transaction<'_, MySql>, player_id: Ulid) -> sqlx::Result<()> {
    // 同じデバイスから同時に作られても1人だけになるように、player_id の主キーに任せる
    sqlx::query(
        "INSERT INTO sandbox_players (player_id) VALUES (?) \
         ON DUPLICATE KEY UPDATE player_id = player_id",
    )
    .bind(player_id.to_bytes().as_slice())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// カウンターを読む。プレイヤーがいなければ `None`。
pub(crate) async fn counter<'e>(
    executor: impl MySqlExecutor<'e>,
    player_id: Ulid,
) -> sqlx::Result<Option<i64>> {
    sqlx::query_scalar("SELECT counter FROM sandbox_players WHERE player_id = ?")
        .bind(player_id.to_bytes().as_slice())
        .fetch_optional(executor)
        .await
}

/// カウンターに `amount` を足し、足した後の値を返す。
pub(crate) async fn increment(
    tx: &mut Transaction<'_, MySql>,
    player_id: Ulid,
    amount: i64,
) -> sqlx::Result<i64> {
    sqlx::query("UPDATE sandbox_players SET counter = counter + ? WHERE player_id = ?")
        .bind(amount)
        .bind(player_id.to_bytes().as_slice())
        .execute(&mut **tx)
        .await?;
    sqlx::query_scalar("SELECT counter FROM sandbox_players WHERE player_id = ?")
        .bind(player_id.to_bytes().as_slice())
        .fetch_one(&mut **tx)
        .await
}
