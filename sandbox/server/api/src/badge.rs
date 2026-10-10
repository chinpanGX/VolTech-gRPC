use jiff::civil::Date;
use player::badge::{BadgeCollector, BadgeKind};
use proto::sandbox::v1::{Badge, BadgeType};
use sqlx::{MySql, Transaction};
use ulid::Ulid;

/// 記録された種類のバッジの、今の状態を計算してレスポンスの `badges` にする。記録が無ければ空。
///
/// バッジの判定はアプリが書く。種類を足したら、ここに判定を足す。
pub(crate) async fn to_proto(
    tx: &mut Transaction<'_, MySql>,
    badges: &BadgeCollector,
    player_id: Ulid,
    today: Date,
) -> sqlx::Result<Vec<Badge>> {
    let mut resolved = Vec::new();
    for kind in badges.kinds() {
        let badge = match kind {
            BadgeKind::FreeGacha => Badge {
                r#type: BadgeType::FreeGacha.into(),
                visible: gacha::service::free_draw_available(&mut **tx, player_id, today).await?,
                count: 0,
            },
            BadgeKind::Counter => {
                let counter = player::service::find_by_id(&mut **tx, player_id)
                    .await?
                    .map_or(0, |player| player.counter);
                counter_badge(counter)
            }
        };
        resolved.push(badge);
    }
    Ok(resolved)
}

/// カウンターのバッジ。0 より大きければ表示し、件数を出す。
pub(crate) fn counter_badge(counter: i64) -> Badge {
    Badge {
        r#type: BadgeType::Counter.into(),
        visible: counter > 0,
        count: i32::try_from(counter).unwrap_or(i32::MAX),
    }
}
