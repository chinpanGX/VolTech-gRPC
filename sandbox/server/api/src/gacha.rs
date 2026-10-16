use gacha::service::{DrawCost, DrawError};
use player::collector::Collectors;
use proto::errors;
use proto::sandbox::v1::gacha_service_server::{GachaService, GachaServiceServer};
use proto::sandbox::v1::{DrawRequest, DrawResponse};
use tonic::{Request, Response, Status};
use voltech_grpc_error::{ResultExt, failed_precondition};
use voltech_grpc_transaction::Transactions;

use crate::context::{game_date, player_id};
use crate::player::item_to_proto;
use crate::{badge, diff, notification};

/// `GachaService` の実装。
#[derive(Clone)]
pub struct GachaServiceImpl {
    transactions: Transactions,
}

impl GachaServiceImpl {
    /// ハンドラーの本体を動かすトランザクションを受け取って作る。
    pub fn new(transactions: Transactions) -> Self {
        Self { transactions }
    }

    /// tonic のルーターに登録できる形にする。`AuthLayer` を付けて登録すること。
    pub fn into_server(self) -> GachaServiceServer<Self> {
        GachaServiceServer::new(self)
    }
}

#[tonic::async_trait]
impl GachaService for GachaServiceImpl {
    async fn draw(&self, request: Request<DrawRequest>) -> Result<Response<DrawResponse>, Status> {
        let player_id = player_id(&request)?;
        let today = game_date(&request)?;
        let cost = if request.get_ref().free {
            DrawCost::Free { today }
        } else {
            DrawCost::Gems
        };

        let response = self
            .transactions
            .run(&request, Some(player_id), move |ctx| {
                Box::pin(async move {
                    let mut collected = Collectors::default();
                    let result = gacha::service::draw(ctx, &mut collected, player_id, cost)
                        .await
                        .map_err(|err| match err {
                            DrawError::InsufficientGems => {
                                failed_precondition(errors::INSUFFICIENT_GEMS, "not enough gems")
                            }
                            DrawError::FreeDrawUsed => failed_precondition(
                                errors::FREE_DRAW_USED,
                                "free draw already used today",
                            ),
                            DrawError::Internal(err) => err.into(),
                        })?;
                    Ok(DrawResponse {
                        item: Some(item_to_proto(result.item)),
                        player_diff: diff::to_proto(&collected.diff),
                        badges: badge::to_proto(ctx.tx(), &collected.badges, player_id, today)
                            .await
                            .or_internal()?,
                        notifications: notification::to_proto(&collected.notifications),
                    })
                })
            })
            .await?;
        Ok(Response::new(response))
    }
}
