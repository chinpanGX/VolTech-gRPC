use sqlx::MySqlExecutor;
use ulid::Ulid;
use voltech_grpc_error::Internal;
use voltech_grpc_transaction::{GameLog, TxContext};

use crate::diff::PlayerDiffCollector;
use crate::model::{Player, PlayerItem};

/// プレイヤーを作ったときに持たせるジェムの数。
pub const INITIAL_GEMS: i64 = 3000;

// ゲームログの種別
const GEMS: &str = "gems";
const ITEM: &str = "item";
const COUNTER: &str = "counter";

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
///
/// 作ったときだけ、初期データのジェムをゲームログに書く。
pub async fn create(ctx: &mut TxContext, player_id: Ulid) -> Result<Player, Internal> {
    // 同じデバイスから同時に作られても1人だけになるように、player_id の主キーに任せる。
    // 作ったかどうかを件数で見分けるため、ON DUPLICATE KEY UPDATE ではなく INSERT IGNORE にする
    // (sqlx は CLIENT_FOUND_ROWS で接続するので、ON DUPLICATE KEY UPDATE では作成済みでも 1 件になる)
    let inserted = sqlx::query("INSERT IGNORE INTO players (player_id, gems) VALUES (?, ?)")
        .bind(player_id.to_bytes().as_slice())
        .bind(INITIAL_GEMS)
        .execute(&mut **ctx.tx())
        .await?
        .rows_affected()
        == 1;
    if inserted {
        ctx.write_game_log(GameLog::new(GEMS, INITIAL_GEMS, INITIAL_GEMS, "sign_up"))?;
    }

    let row: PlayerRow = sqlx::query_as(select_player!("player_id = ?"))
        .bind(player_id.to_bytes().as_slice())
        .fetch_one(&mut **ctx.tx())
        .await?;
    Ok(into_player(row)?)
}

/// ジェムを `amount` だけ消費し、残りを返す。足りなければ消費せずに `None` を返す。
///
/// 消費したら、`reason` を理由にしてゲームログに書く。
pub async fn consume_gems(
    ctx: &mut TxContext,
    diff: &mut PlayerDiffCollector,
    player_id: Ulid,
    amount: i64,
    reason: &str,
) -> Result<Option<i64>, Internal> {
    // 同時に消費されても残りを超えないように、行をロックしてから確かめる
    let gems: i64 = sqlx::query_scalar("SELECT gems FROM players WHERE player_id = ? FOR UPDATE")
        .bind(player_id.to_bytes().as_slice())
        .fetch_one(&mut **ctx.tx())
        .await?;
    if gems < amount {
        return Ok(None);
    }

    sqlx::query("UPDATE players SET gems = gems - ? WHERE player_id = ?")
        .bind(amount)
        .bind(player_id.to_bytes().as_slice())
        .execute(&mut **ctx.tx())
        .await?;
    let remaining = gems - amount;
    ctx.write_game_log(GameLog::new(GEMS, -amount, remaining, reason))?;
    diff.record_gems(remaining);
    Ok(Some(remaining))
}

/// アイテムを `count` 個増やし、増やした後の所持数を返す。`reason` を理由にしてゲームログに書く。
pub async fn add_item(
    ctx: &mut TxContext,
    diff: &mut PlayerDiffCollector,
    player_id: Ulid,
    item_id: i32,
    count: i64,
    reason: &str,
) -> Result<PlayerItem, Internal> {
    sqlx::query(
        "INSERT INTO player_items (player_id, item_id, count) VALUES (?, ?, ?) AS new \
         ON DUPLICATE KEY UPDATE count = player_items.count + new.count",
    )
    .bind(player_id.to_bytes().as_slice())
    .bind(item_id)
    .bind(count)
    .execute(&mut **ctx.tx())
    .await?;

    let after: i64 =
        sqlx::query_scalar("SELECT count FROM player_items WHERE player_id = ? AND item_id = ?")
            .bind(player_id.to_bytes().as_slice())
            .bind(item_id)
            .fetch_one(&mut **ctx.tx())
            .await?;
    ctx.write_game_log(GameLog::new(ITEM, count, after, reason).target(item_id))?;
    let item = PlayerItem {
        item_id,
        count: after,
    };
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

/// カウンターに `amount` を足し、足した後の値を返す。ゲームログにも書く。
pub async fn increment_counter(
    ctx: &mut TxContext,
    diff: &mut PlayerDiffCollector,
    player_id: Ulid,
    amount: i64,
) -> Result<i64, Internal> {
    sqlx::query("UPDATE players SET counter = counter + ? WHERE player_id = ?")
        .bind(amount)
        .bind(player_id.to_bytes().as_slice())
        .execute(&mut **ctx.tx())
        .await?;
    let counter: i64 = sqlx::query_scalar("SELECT counter FROM players WHERE player_id = ?")
        .bind(player_id.to_bytes().as_slice())
        .fetch_one(&mut **ctx.tx())
        .await?;
    ctx.write_game_log(GameLog::new(COUNTER, amount, counter, "increment"))?;
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
