mod context;
mod game_log;
mod idempotency;
mod method;

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use prost::Message;
use sqlx::{MySql, MySqlPool, Transaction};
use tonic::{Request, Status};
use tracing::Span;
use ulid::Ulid;
use voltech_grpc_auth::AuthContext;
use voltech_grpc_error::{Internal, ResultExt, internal};
use voltech_grpc_log::{MAX_REQUEST_ID_LEN, REQUEST_ID_HEADER, field, is_valid_request_id};

use crate::game_log::GameLogOrigin;

pub use context::TxContext;
pub use game_log::{GameLog, GameLogSink};
pub use idempotency::{Idempotency, IdempotencyLevels};
pub use method::{RpcMethodLayer, RpcMethodService};
pub use voltech_grpc_log::RpcMethod;

/// `Transactions::run` に渡す処理が返す Future。`TxContext` を借りている間だけ生きる。
///
/// 処理は `move |ctx| Box::pin(async move { ... })` の形で書く。
pub type HandlerFuture<'t, R> = Pin<Box<dyn Future<Output = Result<R, Status>> + Send + 't>>;

/// ハンドラーの本体をトランザクションの中で動かし、冪等性の対象の RPC では二重実行を防ぐ。
#[derive(Clone)]
pub struct Transactions {
    pool: MySqlPool,
    levels: Arc<IdempotencyLevels>,
    game_log_sink: GameLogSink,
}

impl Transactions {
    /// DB と、RPC ごとの冪等性の扱いの表と、ゲームログの出力先を受け取って作る。
    pub fn new(pool: MySqlPool, levels: IdempotencyLevels, game_log_sink: GameLogSink) -> Self {
        Self {
            pool,
            levels: Arc::new(levels),
            game_log_sink,
        }
    }

    /// `handler` をトランザクションの中で動かし、成功したらコミットする。失敗したらロールバックする。
    ///
    /// RPC の `idempotency_level` によって動きが変わる。
    /// - 書いていない(冪等性の対象): プレイヤーの直前のレスポンスの行をロックし、
    ///   同じ `x-request-id` なら `handler` を呼ばずに保存済みのレスポンスを返す。成功したらレスポンスを保存する
    /// - `NO_SIDE_EFFECTS`: 読み取り専用のトランザクションで動かす。ゲームログは書けない
    /// - `IDEMPOTENT`: 普通のトランザクションで動かす
    ///
    /// 冪等性の対象の RPC では `player_id` が要る(無ければ INTERNAL)。
    /// ゲームログの `player_id` は、この引数ではなく `AuthContext` から取る。
    pub async fn run<T, R>(
        &self,
        request: &Request<T>,
        player_id: Option<Ulid>,
        handler: impl for<'t> FnOnce(&'t mut TxContext) -> HandlerFuture<'t, R> + Send,
    ) -> Result<R, Status>
    where
        R: Message + Default + Send,
    {
        let method = request.extensions().get::<RpcMethod>().ok_or_else(|| {
            internal(Internal::msg(
                "RpcMethod is missing. Is RpcMethodLayer applied to the server?",
            ))
        })?;
        let call = Call {
            method_name: method.full_name(),
            request_id: valid_request_id(request),
            auth_player_id: request
                .extensions()
                .get::<AuthContext>()
                .map(|context| context.player_id),
        };

        match self.levels.get(method.service(), method.method()) {
            Idempotency::Target => {
                let player_id = player_id.ok_or_else(|| {
                    internal(Internal::msg(format!(
                        "{} is an idempotency target but has no player",
                        call.method_name
                    )))
                })?;
                let request_id = call.request_id.as_deref().ok_or_else(|| {
                    Status::invalid_argument(format!(
                        "{REQUEST_ID_HEADER} must be 1..={MAX_REQUEST_ID_LEN} alphanumeric characters"
                    ))
                })?;
                self.run_target(&call, player_id, request_id, handler).await
            }
            Idempotency::NoSideEffects => {
                // 誤って書き込むと MySQL がその場でエラーを返すので、宣言の間違いに開発中に気づける
                let tx = self
                    .pool
                    .begin_with("START TRANSACTION READ ONLY")
                    .await
                    .or_internal()?;
                let mut ctx = TxContext::new(tx, call.auth_player_id, true);
                let response = handler(&mut ctx).await?;
                self.commit(&call, ctx).await?;
                Ok(response)
            }
            Idempotency::Idempotent => {
                let tx = self.pool.begin().await.or_internal()?;
                let mut ctx = TxContext::new(tx, call.auth_player_id, false);
                let response = handler(&mut ctx).await?;
                self.commit(&call, ctx).await?;
                Ok(response)
            }
        }
    }

    async fn run_target<R>(
        &self,
        call: &Call,
        player_id: Ulid,
        request_id: &str,
        handler: impl for<'t> FnOnce(&'t mut TxContext) -> HandlerFuture<'t, R> + Send,
    ) -> Result<R, Status>
    where
        R: Message + Default + Send,
    {
        let mut tx = self.pool.begin().await.or_internal()?;

        // 再送が届いても最初の処理の完了を待てるように、業務データより先に直前のレスポンスの行をロックする
        let last = match lock_last_response(&mut tx, player_id).await? {
            Some(last) => last,
            None => {
                // この仕組みより前に作られたプレイヤーには行が無いので作る。行が無いときの FOR UPDATE は
                // 隙間のロックを取り、それを持ったまま INSERT すると、同じ隙間に INSERT する別のプレイヤーの
                // トランザクションとデッドロックする。ロックを手放してから、別のトランザクションで作る
                tx.rollback().await.or_internal()?;
                let mut create = self.pool.begin().await.or_internal()?;
                create_last_response(&mut create, player_id)
                    .await
                    .or_internal()?;
                create.commit().await.or_internal()?;

                tx = self.pool.begin().await.or_internal()?;
                lock_last_response(&mut tx, player_id)
                    .await?
                    .ok_or_else(|| {
                        internal(Internal::msg(
                            "last_responses row is missing after it was created",
                        ))
                    })?
            }
        };
        if let Some((saved_request_id, saved_method, saved_response)) = last
            && saved_request_id == request_id
        {
            if saved_method != call.method_name {
                // クライアントの不具合で RequestID が使い回されたとき、型の違うレスポンスを返さない
                tracing::warn!(
                    saved_method,
                    "x-request-id was already used for another RPC"
                );
                return Err(Status::invalid_argument(format!(
                    "x-request-id was already used for {saved_method}"
                )));
            }
            Span::current().record(field::REPLAYED, true);
            return R::decode(saved_response.as_slice()).or_internal();
        }
        Span::current().record(field::REPLAYED, false);

        let mut ctx = TxContext::new(tx, call.auth_player_id, false);
        let response = handler(&mut ctx).await?;

        sqlx::query(
            "UPDATE last_responses SET request_id = ?, method = ?, response = ? WHERE player_id = ?",
        )
        .bind(request_id)
        .bind(&call.method_name)
        .bind(response.encode_to_vec())
        .bind(player_id.to_bytes().as_slice())
        .execute(&mut **ctx.tx())
        .await
        .or_internal()?;
        self.commit(call, ctx).await?;
        Ok(response)
    }

    /// ゲームログを出力先へ書いてコミットする。標準出力へは、コミットに成功してから出す。
    async fn commit(&self, call: &Call, ctx: TxContext) -> Result<(), Status> {
        let (mut tx, game_logs) = ctx.into_parts();
        let game_logs = game_logs.map(|(player_id, logs)| {
            let origin = GameLogOrigin {
                player_id,
                request_id: call.request_id.clone(),
                method: call.method_name.clone(),
            };
            (origin, logs)
        });

        if self.game_log_sink == GameLogSink::Database
            && let Some((origin, logs)) = &game_logs
        {
            game_log::write_to_database(&mut tx, origin, logs)
                .await
                .or_internal()?;
        }
        tx.commit().await.or_internal()?;
        if self.game_log_sink == GameLogSink::Stdout
            && let Some((origin, logs)) = &game_logs
        {
            game_log::write_to_stdout(origin, logs);
        }
        Ok(())
    }
}

/// 1回の `run` の呼び出しで、RPC から読んだ値。
struct Call {
    method_name: String,
    /// 形の正しい `x-request-id`。
    request_id: Option<String>,
    /// `AuthLayer` が認証したプレイヤー。ゲームログに付ける。
    auth_player_id: Option<Ulid>,
}

/// プレイヤーの直前のレスポンスを保存する行を作る。プレイヤーを作るときに、同じトランザクションで呼ぶ。作成済みなら何もしない。
pub async fn create_last_response(
    tx: &mut Transaction<'_, MySql>,
    player_id: Ulid,
) -> sqlx::Result<()> {
    sqlx::query("INSERT IGNORE INTO last_responses (player_id) VALUES (?)")
        .bind(player_id.to_bytes().as_slice())
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// 基盤のテーブル(`last_responses`・`game_logs`)を作成・更新する。アプリの起動時に呼ぶ。
pub async fn migrate(pool: &MySqlPool) -> Result<(), sqlx::migrate::MigrateError> {
    let mut migrator = sqlx::migrate!("./migrations");
    // アプリや他の基盤のマイグレーションと履歴を同じテーブルに入れると、互いに相手の分を
    // 「知らないマイグレーションが適用済み」とみなしてエラーになるので分ける
    migrator.dangerous_set_table_name("_transaction_migrations");
    migrator.run(pool).await
}

type LastResponse = (String, String, Vec<u8>);

/// プレイヤーの直前のレスポンスの行をロックして読む。行が無ければ `None`、行はあるがまだ保存していなければ `Some(None)`。
async fn lock_last_response(
    tx: &mut Transaction<'static, MySql>,
    player_id: Ulid,
) -> Result<Option<Option<LastResponse>>, Status> {
    let row = sqlx::query_as::<_, (Option<String>, Option<String>, Option<Vec<u8>>)>(
        "SELECT request_id, method, response FROM last_responses WHERE player_id = ? FOR UPDATE",
    )
    .bind(player_id.to_bytes().to_vec())
    .fetch_optional(&mut **tx)
    .await
    .or_internal()?;

    Ok(row.map(|row| match row {
        (Some(request_id), Some(method), Some(response)) => Some((request_id, method, response)),
        _ => None,
    }))
}

fn valid_request_id<T>(request: &Request<T>) -> Option<String> {
    request
        .metadata()
        .get(REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .filter(|request_id| is_valid_request_id(request_id))
        .map(str::to_owned)
}
