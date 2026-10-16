use jiff::civil::Date;
use player::badge::BadgeKind;
use player::collector::Collectors;
use player::model::PlayerItem;
use player::notification::Notification;
use sqlx::{Executor, MySql};
use ulid::Ulid;
use voltech_grpc_error::Internal;
use voltech_grpc_transaction::{GameLog, TxContext};

use crate::model::{DRAW_COST, pick, total_weight};

// ゲームログの理由
const GACHA_DRAW: &str = "gacha_draw";
// ゲームログの種別: 1日1回の無料ガチャを使った
const FREE_DRAW: &str = "free_draw";

/// ガチャを引いた結果。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrawResult {
    /// 排出されたアイテム(付与した後の個数)。
    pub item: PlayerItem,
}

/// ガチャの引き方。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawCost {
    /// ジェムを `DRAW_COST` だけ使う。
    Gems,
    /// 1日1回の無料ガチャ。`today` はゲーム内の日付。
    Free { today: Date },
}

/// ガチャを引けなかった理由。
#[derive(Debug)]
pub enum DrawError {
    /// ジェムが足りない。
    InsufficientGems,
    /// 今日の無料ガチャはもう引いた。
    FreeDrawUsed,
    Internal(Internal),
}

impl<E: Into<Internal>> From<E> for DrawError {
    #[track_caller]
    fn from(err: E) -> Self {
        Self::Internal(err.into())
    }
}

/// ガチャを1回引き、排出されたアイテムを付与する。引けなければ何も変えない。
///
/// 変わったもの(ジェム・アイテム)は `collected.diff` とゲームログに、無料ガチャのバッジは `collected.badges` に、
/// レアが出たことは `collected.notifications` に記録する。
pub async fn draw(
    ctx: &mut TxContext,
    collected: &mut Collectors,
    player_id: Ulid,
    cost: DrawCost,
) -> Result<DrawResult, DrawError> {
    match cost {
        DrawCost::Gems => {
            player::service::consume_gems(
                ctx,
                &mut collected.diff,
                player_id,
                DRAW_COST,
                GACHA_DRAW,
            )
            .await?
            .ok_or(DrawError::InsufficientGems)?;
        }
        DrawCost::Free { today } => {
            if !free_draw_available(&mut **ctx.tx(), player_id, today).await? {
                return Err(DrawError::FreeDrawUsed);
            }
            record_free_draw(ctx, player_id, today).await?;
            collected.badges.mark(BadgeKind::FreeGacha);
        }
    }

    let drawn = pick(rand::random_range(0..total_weight()));
    let item = player::service::add_item(
        ctx,
        &mut collected.diff,
        player_id,
        drawn.item_id,
        1,
        GACHA_DRAW,
    )
    .await?;
    if drawn.rare {
        collected
            .notifications
            .push(Notification::RareItemObtained {
                item_id: drawn.item_id,
            });
    }
    Ok(DrawResult { item })
}

/// ゲーム内の日付 `today` の無料ガチャを、まだ引いていないか。無料ガチャのバッジの判定にも使う。
pub async fn free_draw_available<'e>(
    executor: impl Executor<'e, Database = MySql>,
    player_id: Ulid,
    today: Date,
) -> sqlx::Result<bool> {
    let drawn: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM gacha_free_draws WHERE player_id = ? AND last_drawn_on >= ?",
    )
    .bind(player_id.to_bytes().as_slice())
    .bind(today.to_string())
    .fetch_one(executor)
    .await?;
    Ok(drawn == 0)
}

// 無料ガチャを引いた日を記録する。同じプレイヤーの処理は、冪等性の行のロックで1つずつ動くので、
// free_draw_available で確かめてから書いても、同時に2回引かれることはない
async fn record_free_draw(
    ctx: &mut TxContext,
    player_id: Ulid,
    today: Date,
) -> Result<(), Internal> {
    let today = today.to_string();
    sqlx::query(
        "INSERT INTO gacha_free_draws (player_id, last_drawn_on) VALUES (?, ?) \
         ON DUPLICATE KEY UPDATE last_drawn_on = ?",
    )
    .bind(player_id.to_bytes().as_slice())
    .bind(&today)
    .bind(&today)
    .execute(&mut **ctx.tx())
    .await?;
    // 数量の変化でないので、増減を持たないゲームログにし、使った日を名前付きの値で残す
    ctx.write_game_log(GameLog::event(FREE_DRAW, GACHA_DRAW).field("drawn_on", today))?;
    Ok(())
}
