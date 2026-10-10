use proto::errors;
use proto::sandbox::v1::notification::Event;
use proto::sandbox::v1::sandbox_service_server::{SandboxService, SandboxServiceServer};
use proto::sandbox::v1::{
    Badge, BadgeType, FailRequest, FailResponse, IncrementRequest, IncrementResponse, Notification,
    PingReceived, PingRequest, PingResponse, PlayerDiff, SignUpRequest, SignUpResponse,
};
use tonic::{Request, Response, Status};
use ulid::Ulid;
use voltech_grpc_auth::AuthContext;
use voltech_grpc_error::{failed_precondition, internal, reserved};
use voltech_grpc_transaction::Transactions;

use crate::players;

/// `SandboxService` の実装。基盤の機能を1つずつ試すためだけの RPC。
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
    async fn sign_up(
        &self,
        request: Request<SignUpRequest>,
    ) -> Result<Response<SignUpResponse>, Status> {
        // サインアップの前なので、作成済みかは問わずに、デバイス登録で発行した player_id を使う
        let player_id = auth_context(&request)?.player_id;

        let response = self
            .transactions
            .run(&request, None, move |tx| {
                Box::pin(async move {
                    players::create(tx, player_id).await.map_err(internal)?;
                    // 冪等性の対象の RPC でロックする行を、プレイヤーと一緒に作っておく
                    voltech_grpc_transaction::create_last_response(tx, player_id)
                        .await
                        .map_err(internal)?;
                    Ok(SignUpResponse {
                        player_id: player_id.to_string(),
                    })
                })
            })
            .await?;
        Ok(Response::new(response))
    }

    async fn ping(&self, request: Request<PingRequest>) -> Result<Response<PingResponse>, Status> {
        let player_id = player_id(&request)?;

        let response = self
            .transactions
            .run(&request, Some(player_id), move |tx| {
                Box::pin(async move {
                    let counter = players::counter(&mut **tx, player_id)
                        .await
                        .map_err(internal)?
                        .ok_or_else(player_not_found)?;
                    Ok(PingResponse {
                        player_id: player_id.to_string(),
                        player_diff: Some(PlayerDiff {
                            counter: Some(counter),
                        }),
                        // Ping ではすべての種類のバッジを返す
                        badges: vec![counter_badge(counter)],
                        notifications: vec![Notification {
                            event: Some(Event::PingReceived(PingReceived {})),
                        }],
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
                    let counter = players::increment(tx, player_id, amount)
                        .await
                        .map_err(internal)?;
                    Ok(IncrementResponse {
                        counter,
                        player_diff: Some(PlayerDiff {
                            counter: Some(counter),
                        }),
                    })
                })
            })
            .await?;
        Ok(Response::new(response))
    }
}

/// `AuthLayer` が入れた認証の情報を取り出す。
fn auth_context<T>(request: &Request<T>) -> Result<&AuthContext, Status> {
    // AuthLayer を付け忘れたときだけ無い。クライアントには原因が無いので INTERNAL にする
    request
        .extensions()
        .get::<AuthContext>()
        .ok_or_else(|| internal("AuthContext is missing. Is AuthLayer applied to this service?"))
}

/// サインアップ済みのプレイヤーの ID を取り出す。まだなら PLAYER_NOT_FOUND にする。
fn player_id<T>(request: &Request<T>) -> Result<Ulid, Status> {
    let context = auth_context(request)?;
    if context.player_created {
        Ok(context.player_id)
    } else {
        Err(player_not_found())
    }
}

/// プレイヤーがまだ作られていないことを表す業務エラーを作る。
fn player_not_found() -> Status {
    failed_precondition(errors::PLAYER_NOT_FOUND, "player is not created yet")
}

/// カウンターのバッジ。0 より大きければ表示し、件数を出す。
fn counter_badge(counter: i64) -> Badge {
    Badge {
        r#type: BadgeType::Counter.into(),
        visible: counter > 0,
        count: i32::try_from(counter).unwrap_or(i32::MAX),
    }
}
