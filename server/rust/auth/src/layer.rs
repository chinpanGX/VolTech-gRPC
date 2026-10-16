use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use http::HeaderMap;
use sqlx::MySqlPool;
use tonic::Status;
use tonic::server::NamedService;
use tower::{Layer, Service};
use tracing::Span;

use crate::resolver::{PlayerId, PlayerResolver, PlayerStatus};
use voltech_grpc_error::{Internal, ResultExt, failed_precondition, internal};
use voltech_grpc_log::field;

/// 認証済みのリクエストに付く情報。ハンドラーは `request.extensions()` から取り出す。
#[derive(Clone, Debug)]
pub struct AuthContext {
    pub device_id: String,
    /// デバイス登録で発行した ID。サインアップの前から必ずある。
    pub player_id: PlayerId,
    /// サインアップ済み(プレイヤーが作られている)か。
    pub player_created: bool,
}

/// メタデータ `authorization: Bearer <token>` を検証する tower の Layer。
/// 認証が必要なサービスにだけ付ける。
#[derive(Clone)]
pub struct AuthLayer<R> {
    pool: MySqlPool,
    resolver: R,
}

impl<R: PlayerResolver> AuthLayer<R> {
    /// トークンの検証に使う DB と、プレイヤーを引く処理を受け取って Layer を作る。
    pub fn new(pool: MySqlPool, resolver: R) -> Self {
        Self { pool, resolver }
    }
}

impl<S, R: Clone> Layer<S> for AuthLayer<R> {
    type Service = AuthService<S, R>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthService {
            inner,
            pool: self.pool.clone(),
            resolver: self.resolver.clone(),
        }
    }
}

/// `AuthLayer` を付けたサービス。
#[derive(Clone)]
pub struct AuthService<S, R> {
    inner: S,
    pool: MySqlPool,
    resolver: R,
}

impl<S: NamedService, R> NamedService for AuthService<S, R> {
    const NAME: &'static str = S::NAME;
}

impl<S, R, ReqBody, ResBody> Service<http::Request<ReqBody>> for AuthService<S, R>
where
    S: Service<http::Request<ReqBody>, Response = http::Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send,
    R: PlayerResolver,
    ReqBody: Send + 'static,
    ResBody: Default,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut request: http::Request<ReqBody>) -> Self::Future {
        // poll_ready で準備ができたのは self.inner なので、そちらを持ち出して代わりに複製を残す
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
        let pool = self.pool.clone();
        let resolver = self.resolver.clone();

        Box::pin(async move {
            match authenticate(&pool, &resolver, request.headers()).await {
                Ok(context) => {
                    request.extensions_mut().insert(context);
                    inner.call(request).await
                }
                Err(status) => Ok(status.into_http()),
            }
        })
    }
}

async fn authenticate<R: PlayerResolver>(
    pool: &MySqlPool,
    resolver: &R,
    headers: &HeaderMap,
) -> Result<AuthContext, Status> {
    let Some(token) = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
    else {
        tracing::warn!(reason = "missing_token", "authentication failed");
        return Err(Status::unauthenticated("missing access token"));
    };

    // トークンから device_id と、デバイス登録で発行した player_id を1回で引く。
    // 期限切れと、知らないトークンをログで見分けるため、期限は WHERE ではなく値として読む
    let row: Option<(String, Vec<u8>, i64)> = sqlx::query_as(
        "SELECT d.device_id, d.player_id, t.expires_at > NOW(3) FROM access_tokens t \
         JOIN devices d ON d.device_id = t.device_id \
         WHERE t.token = ?",
    )
    .bind(token)
    .fetch_optional(pool)
    .await
    .or_internal()?;
    let (device_id, player_id) = match row {
        Some((device_id, player_id, 1)) => (device_id, player_id),
        Some((device_id, _, _)) => {
            // 期限切れは、クライアントがトークンを発行し直す前の普段の動作でも起きるので warn にしない
            tracing::info!(reason = "expired_token", device_id, "authentication failed");
            return Err(invalid_token());
        }
        None => {
            // 発行し直す前の古いトークンもここに来る(発行し直すと行を上書きするため)
            tracing::info!(reason = "unknown_token", "authentication failed");
            return Err(invalid_token());
        }
    };
    let player_id: [u8; 16] = player_id.try_into().map_err(|bytes: Vec<u8>| {
        internal(Internal::msg(format!(
            "player_id must be 16 bytes, got {}",
            bytes.len()
        )))
    })?;
    let player_id = PlayerId::from_bytes(player_id);
    Span::current().record(field::PLAYER_ID, player_id.to_string());

    let player_created = match resolver
        .resolve(player_id)
        .await
        .map_err(|err| internal(Internal::from_boxed(err)))?
    {
        PlayerStatus::NotCreated => false,
        PlayerStatus::Active => true,
        PlayerStatus::Suspended => {
            return Err(failed_precondition(
                voltech_grpc_error::reserved::ACCOUNT_SUSPENDED,
                "account suspended",
            ));
        }
    };

    Ok(AuthContext {
        device_id,
        player_id,
        player_created,
    })
}

fn invalid_token() -> Status {
    Status::unauthenticated("invalid or expired access token")
}
