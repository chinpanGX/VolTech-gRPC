use player::badge::BadgeKind;
use player::collector::Collectors;
use player::notification::Notification as Event;
use proto::errors;
use proto::sandbox::v1::sandbox_service_server::{SandboxService, SandboxServiceServer};
use proto::sandbox::v1::{
    FailRequest, FailResponse, IncrementRequest, IncrementResponse, PingRequest, PingResponse,
};
use tonic::{Request, Response, Status};
use voltech_grpc_error::{failed_precondition, internal, reserved};
use voltech_grpc_transaction::Transactions;

use crate::context::{game_date, player_id, player_not_found};
use crate::{badge, diff, notification};

/// `SandboxService` の実装。基盤の機能を1つずつ試すための RPC。
#[derive(Clone)]
pub struct SandboxServiceImpl {
    transactions: Transactions,
}

impl SandboxServiceImpl {
    /// ハンドラーの本体を動かすトランザクションを受け取って作る。
    pub fn new(transactions: Transactions) -> Self {
        Self { transactions }
    }

    /// tonic のルーターに登録できる形にする。`AuthLayer` を付けて登録すること。
    pub fn into_server(self) -> SandboxServiceServer<Self> {
        SandboxServiceServer::new(self)
    }
}

#[tonic::async_trait]
impl SandboxService for SandboxServiceImpl {
    async fn ping(&self, request: Request<PingRequest>) -> Result<Response<PingResponse>, Status> {
        let player_id = player_id(&request)?;
        let today = game_date(&request)?;

        let response = self
            .transactions
            .run(&request, Some(player_id), move |tx| {
                Box::pin(async move {
                    let player = player::service::find_by_id(&mut **tx, player_id)
                        .await
                        .map_err(internal)?
                        .ok_or_else(player_not_found)?;
                    let mut collected = Collectors::default();
                    // Ping ではカウンターの現在値・バッジと、疎通できたことの通知を返す
                    collected.diff.record_counter(player.counter);
                    collected.badges.mark(BadgeKind::Counter);
                    collected.notifications.push(Event::PingReceived);
                    Ok(PingResponse {
                        player_id: player_id.to_string(),
                        player_diff: diff::to_proto(&collected.diff),
                        badges: badge::to_proto(tx, &collected.badges, player_id, today)
                            .await
                            .map_err(internal)?,
                        notifications: notification::to_proto(&collected.notifications),
                    })
                })
            })
            .await?;
        Ok(Response::new(response))
    }

    async fn fail(&self, request: Request<FailRequest>) -> Result<Response<FailResponse>, Status> {
        let FailRequest { domain, reason } = request.get_ref();
        let code = if reason.is_empty() {
            reserved::INTERNAL_ERROR
        } else {
            *errors::ALL
                .iter()
                .chain(reserved::ALL)
                .find(|code| code.domain() == domain && code.reason() == reason)
                .ok_or_else(|| {
                    Status::invalid_argument(format!("unknown error code: {domain}/{reason}"))
                })?
        };
        Err(failed_precondition(
            code,
            "failed on purpose by SandboxService/Fail",
        ))
    }

    async fn increment(
        &self,
        request: Request<IncrementRequest>,
    ) -> Result<Response<IncrementResponse>, Status> {
        let player_id = player_id(&request)?;
        let amount = request.get_ref().amount;

        let response = self
            .transactions
            .run(&request, Some(player_id), move |tx| {
                Box::pin(async move {
                    let mut collected = Collectors::default();
                    let counter = player::service::increment_counter(
                        tx,
                        &mut collected.diff,
                        player_id,
                        amount,
                    )
                    .await
                    .map_err(internal)?;
                    Ok(IncrementResponse {
                        counter,
                        player_diff: diff::to_proto(&collected.diff),
                    })
                })
            })
            .await?;
        Ok(Response::new(response))
    }
}
