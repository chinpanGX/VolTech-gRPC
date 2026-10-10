use sqlx::{MySql, MySqlExecutor, Transaction};
use ulid::Ulid;

use crate::diff::PlayerDiffCollector;
use crate::model::{Player, PlayerItem};

/// プレイヤーを作ったときに持たせるジェムの数。
pub const INITIAL_GEMS: i64 = 3000;

// sqlx は SQL を &'static str で受け取るので、format! ではなく concat! で組み立てる
macro_rules! select_player {
    ($condition:literal) => {
        concat!(
            "SELECT player_id, gems, counter, suspended_at IS NOT NULL FROM players WHERE ",
            $condition
        )
    };
}

// player_id は BINARY(16) で持つので、バイト列で読む
type PlayerRow = (Vec<u8>, i64, i64, i64);

/// `player_id` のプレイヤーを探す。いなければ `None` を返す。
pub async fn find_by_id<'e>(
    executor: impl MySqlExecutor<'e>,
    player_id: Ulid,
) -> sqlx::Result<Option<Player>> {
    let row: Option<PlayerRow> = sqlx::query_as(select_player!("player_id = ?"))
        .bind(player_id.to_bytes().as_slice())
        .fetch_optional(executor)
        .await?;
    row.map(into_player).transpose()
}

/// デバイス登録で発行した `player_id` で、プレイヤーと初期データを作る。作成済みなら作らずに、作成済みのプレイヤーを返す。
pub async fn create(tx: &mut Transaction<'_, MySql>, player_id: Ulid) -> sqlx::Result<Player> {
    // 同じデバイスから同時に作られても1人だけになるように、player_id の主キーに任せる
    sqlx::query(
        "INSERT INTO players (player_id, gems) VALUES (?, ?) \
         ON DUPLICATE KEY UPDATE player_id = player_id",
    )
    .bind(player_id.to_bytes().as_slice())
    .bind(INITIAL_GEMS)
    .execute(&mut **tx)
    .await?;

    let row: PlayerRow = sqlx::query_as(select_player!("player_id = ?"))
        .bind(player_id.to_bytes().as_slice())
        .fetch_one(&mut **tx)
        .await?;
    into_player(row)
}

/// ジェムを `amount` だけ消費し、残りを返す。足りなければ消費せずに `None` を返す。
pub async fn consume_gems(
    tx: &mut Transaction<'_, MySql>,
    diff: &mut PlayerDiffCollector,
    player_id: Ulid,
    amount: i64,
) -> sqlx::Result<Option<i64>> {
    // 同時に消費されても残りを超えないように、行をロックしてから確かめる
    let gems: i64 = sqlx::query_scalar("SELECT gems FROM players WHERE player_id = ? FOR UPDATE")
        .bind(player_id.to_bytes().as_slice())
        .fetch_one(&mut **tx)
        .await?;
    if gems < amount {
        return Ok(None);
    }

    sqlx::query("UPDATE players SET gems = gems - ? WHERE player_id = ?")
        .bind(amount)
        .bind(player_id.to_bytes().as_slice())
        .execute(&mut **tx)
        .await?;
    diff.record_gems(gems - amount);
    Ok(Some(gems - amount))
}

/// アイテムを `count` 個増やし、増やした後の所持数を返す。
pub async fn add_item(
    tx: &mut Transaction<'_, MySql>,
    diff: &mut PlayerDiffCollector,
    player_id: Ulid,
    item_id: i32,
    count: i64,
) -> sqlx::Result<PlayerItem> {
    sqlx::query(
        "INSERT INTO player_items (player_id, item_id, count) VALUES (?, ?, ?) AS new \
         ON DUPLICATE KEY UPDATE count = player_items.count + new.count",
    )
    .bind(player_id.to_bytes().as_slice())
    .bind(item_id)
    .bind(count)
    .execute(&mut **tx)
    .await?;

    let count: i64 =
        sqlx::query_scalar("SELECT count FROM player_items WHERE player_id = ? AND item_id = ?")
            .bind(player_id.to_bytes().as_slice())
            .bind(item_id)
            .fetch_one(&mut **tx)
            .await?;
    let item = PlayerItem { item_id, count };
    diff.record_item(item);
    Ok(item)
}

/// 所持しているアイテムをすべて、item_id の順に返す。個数が0のものは含まない。
pub async fn list_items<'e>(
    executor: impl MySqlExecutor<'e>,
    player_id: Ulid,
) -> sqlx::Result<Vec<PlayerItem>> {
    let rows: Vec<(i32, i64)> = sqlx::query_as(
        "SELECT item_id, count FROM player_items WHERE player_id = ? AND count > 0 ORDER BY item_id",
    )
    .bind(player_id.to_bytes().as_slice())
    .fetch_all(executor)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(item_id, count)| PlayerItem { item_id, count })
        .collect())
}

/// カウンターに `amount` を足し、足した後の値を返す。
pub async fn increment_counter(
    tx: &mut Transaction<'_, MySql>,
    diff: &mut PlayerDiffCollector,
    player_id: Ulid,
    amount: i64,
) -> sqlx::Result<i64> {
    sqlx::query("UPDATE players SET counter = counter + ? WHERE player_id = ?")
        .bind(amount)
        .bind(player_id.to_bytes().as_slice())
        .execute(&mut **tx)
        .await?;
    let counter: i64 = sqlx::query_scalar("SELECT counter FROM players WHERE player_id = ?")
        .bind(player_id.to_bytes().as_slice())
        .fetch_one(&mut **tx)
        .await?;
    diff.record_counter(counter);
    Ok(counter)
}

fn into_player((player_id, gems, counter, suspended): PlayerRow) -> sqlx::Result<Player> {
    let player_id: [u8; 16] = player_id.try_into().map_err(|bytes: Vec<u8>| {
        sqlx::Error::Decode(format!("player_id must be 16 bytes, got {}", bytes.len()).into())
    })?;
    Ok(Player {
        player_id: Ulid::from_bytes(player_id),
        gems,
        counter,
        suspended: suspended != 0,
    })
}
