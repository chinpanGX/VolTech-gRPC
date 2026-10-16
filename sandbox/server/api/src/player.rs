use player::badge::BadgeCollector;
use proto::sandbox::v1::player_service_server::{PlayerService, PlayerServiceServer};
use proto::sandbox::v1::{
    Player, PlayerItem, SignInRequest, SignInResponse, SignUpRequest, SignUpResponse,
};
use tonic::{Request, Response, Status};
use voltech_grpc_error::ResultExt;
use voltech_grpc_transaction::Transactions;

use crate::badge;
use crate::context::{auth_context, game_date, player_id, player_not_found};

/// `PlayerService`(サインアップ・サインイン)の実装。
#[derive(Clone)]
pub struct PlayerServiceImpl {
    transactions: Transactions,
}

impl PlayerServiceImpl {
    /// ハンドラーの本体を動かすトランザクションを受け取って作る。
    pub fn new(transactions: Transactions) -> Self {
        Self { transactions }
    }

    /// tonic のルーターに登録できる形にする。`AuthLayer` を付けて登録すること。
    pub fn into_server(self) -> PlayerServiceServer<Self> {
        PlayerServiceServer::new(self)
    }
}

#[tonic::async_trait]
impl PlayerService for PlayerServiceImpl {
    async fn sign_up(
        &self,
        request: Request<SignUpRequest>,
    ) -> Result<Response<SignUpResponse>, Status> {
        // サインアップの前なので、作成済みかは問わずに、デバイス登録で発行した player_id を使う
        let player_id = auth_context(&request)?.player_id;

        let response = self
            .transactions
            .run(&request, None, move |ctx| {
                Box::pin(async move {
                    let player = player::service::create(ctx, player_id).await?;
                    // 冪等性の対象の RPC でロックする行を、プレイヤーと一緒に作っておく
                    voltech_grpc_transaction::create_last_response(ctx.tx(), player.player_id)
                        .await
                        .or_internal()?;
                    Ok(SignUpResponse {
                        player: Some(to_proto(player)),
                    })
                })
            })
            .await?;
        Ok(Response::new(response))
    }

    async fn sign_in(
        &self,
        request: Request<SignInRequest>,
    ) -> Result<Response<SignInResponse>, Status> {
        let player_id = player_id(&request)?;
        let today = game_date(&request)?;

        let response = self
            .transactions
            .run(&request, Some(player_id), move |ctx| {
                Box::pin(async move {
                    let player = player::service::find_by_id(&mut **ctx.tx(), player_id)
                        .await
                        .or_internal()?
                        .ok_or_else(player_not_found)?;
                    let items = player::service::list_items(&mut **ctx.tx(), player_id)
                        .await
                        .or_internal()?;
                    // サインインでは全種類のバッジを返す(運営の操作で変わるバッジも、ここで更新される)
                    let mut badges = BadgeCollector::default();
                    badges.mark_all();
                    Ok(SignInResponse {
                        player: Some(to_proto(player)),
                        items: items.into_iter().map(item_to_proto).collect(),
                        badges: badge::to_proto(ctx.tx(), &badges, player_id, today)
                            .await
                            .or_internal()?,
                    })
                })
            })
            .await?;
        Ok(Response::new(response))
    }
}

fn to_proto(player: player::model::Player) -> Player {
    Player {
        player_id: player.player_id.to_string(),
        gems: player.gems,
        counter: player.counter,
    }
}

/// 所持アイテムを `.proto` の型にする。
pub(crate) fn item_to_proto(item: player::model::PlayerItem) -> PlayerItem {
    PlayerItem {
        item_id: item.item_id,
        count: item.count,
    }
}
