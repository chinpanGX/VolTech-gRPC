mod idempotency;
mod method;

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use prost::Message;
use sqlx::{MySql, MySqlPool, Transaction};
use tonic::{Request, Status};
use ulid::Ulid;
use voltech_grpc_error::internal;

pub use idempotency::{Idempotency, IdempotencyLevels};
pub use method::{RpcMethod, RpcMethodLayer, RpcMethodService};

/// `Transactions::run` に渡す処理が返す Future。トランザクションを借りている間だけ生きる。
///
/// 処理は `move |tx| Box::pin(async move { ... })` の形で書く。
pub type HandlerFuture<'t, R> = Pin<Box<dyn Future<Output = Result<R, Status>> + Send + 't>>;

const REQUEST_ID_HEADER: &str = "x-request-id";
const MAX_REQUEST_ID_LEN: usize = 64;

/// ハンドラーの本体をトランザクションの中で動かし、冪等性の対象の RPC では二重実行を防ぐ。
#[derive(Clone)]
pub struct Transactions {
    pool: MySqlPool,
    levels: Arc<IdempotencyLevels>,
}

impl Transactions {
    /// DB と、RPC ごとの冪等性の扱いの表を受け取って作る。
    pub fn new(pool: MySqlPool, levels: IdempotencyLevels) -> Self {
        Self {
            pool,
            levels: Arc::new(levels),
        }
    }

    /// `handler` をトランザクションの中で動かし、成功したらコミットする。失敗したらロールバックする。
    ///
    /// RPC の `idempotency_level` によって動きが変わる。
    /// - 書いていない(冪等性の対象): プレイヤーの直前のレスポンスの行をロックし、
    ///   同じ `x-request-id` なら `handler` を呼ばずに保存済みのレスポンスを返す。成功したらレスポンスを保存する
    /// - `NO_SIDE_EFFECTS`: 読み取り専用のトランザクションで動かす
    /// - `IDEMPOTENT`: 普通のトランザクションで動かす
    ///
    /// 冪等性の対象の RPC では `player_id` が要る(無ければ INTERNAL)。
    pub async fn run<T, R>(
        &self,
        request: &Request<T>,
        player_id: Option<Ulid>,
        handler: impl for<'t> FnOnce(&'t mut Transaction<'static, MySql>) -> HandlerFuture<'t, R> + Send,
    ) -> Result<R, Status>
    where
        R: Message + Default + Send,
    {
        let method = request.extensions().get::<RpcMethod>().ok_or_else(|| {
            internal("RpcMethod is missing. Is RpcMethodLayer applied to the server?")
        })?;
        let method_name = format!("{}/{}", method.service(), method.method());

        match self.levels.get(method.service(), method.method()) {
            Idempotency::Target => {
                let player_id = player_id.ok_or_else(|| {
                    internal(format!(
                        "{method_name} is an idempotency target but has no player"
                    ))
                })?;
                let request_id = request_id(request)?;
                self.run_target(player_id, &method_name, request_id, handler)
                    .await
            }
            Idempotency::NoSideEffects => {
                // 誤って書き込むと MySQL がその場でエラーを返すので、宣言の間違いに開発中に気づける
                let mut tx = self
                    .pool
                    .begin_with("START TRANSACTION READ ONLY")
                    .await
                    .map_err(internal)?;
                let response = handler(&mut tx).await?;
                tx.commit().await.map_err(internal)?;
                Ok(response)
            }
            Idempotency::Idempotent => {
                let mut tx = self.pool.begin().await.map_err(internal)?;
                let response = handler(&mut tx).await?;
                tx.commit().await.map_err(internal)?;
                Ok(response)
            }
        }
    }

    async fn run_target<R>(
        &self,
        player_id: Ulid,
        method_name: &str,
        request_id: &str,
        handler: impl for<'t> FnOnce(&'t mut Transaction<'static, MySql>) -> HandlerFuture<'t, R> + Send,
    ) -> Result<R, Status>
    where
        R: Message + Default + Send,
    {
        let mut tx = self.pool.begin().await.map_err(internal)?;

        // 再送が届いても最初の処理の完了を待てるように、業務データより先に直前のレスポンスの行をロックする
        let last = match lock_last_response(&mut tx, player_id).await? {
            Some(last) => last,
            None => {
                // この仕組みより前に作られたプレイヤーには行が無いので作る。行が無いときの FOR UPDATE は
                // 隙間のロックを取り、それを持ったまま INSERT すると、同じ隙間に INSERT する別のプレイヤーの
                // トランザクションとデッドロックする。ロックを手放してから、別のトランザクションで作る
                tx.rollback().await.map_err(internal)?;
                let mut create = self.pool.begin().await.map_err(internal)?;
                create_last_response(&mut create, player_id)
                    .await
                    .map_err(internal)?;
                create.commit().await.map_err(internal)?;

                tx = self.pool.begin().await.map_err(internal)?;
                lock_last_response(&mut tx, player_id)
                    .await?
                    .ok_or_else(|| internal("last_responses row is missing after it was created"))?
            }
        };
        if let Some((saved_request_id, saved_method, saved_response)) = last
            && saved_request_id == request_id
        {
            if saved_method != method_name {
                // クライアントの不具合で RequestID が使い回されたとき、型の違うレスポンスを返さない
                return Err(Status::invalid_argument(format!(
                    "x-request-id was already used for {saved_method}"
                )));
            }
            return R::decode(saved_response.as_slice()).map_err(internal);
        }

        let response = handler(&mut tx).await?;

        sqlx::query(
            "UPDATE last_responses SET request_id = ?, method = ?, response = ? WHERE player_id = ?",
        )
        .bind(request_id)
        .bind(method_name)
        .bind(response.encode_to_vec())
        .bind(player_id.to_bytes().as_slice())
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
        tx.commit().await.map_err(internal)?;
        Ok(response)
    }
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

/// 基盤のテーブル(`last_responses`)を作成・更新する。アプリの起動時に呼ぶ。
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
    .map_err(internal)?;

    Ok(row.map(|row| match row {
        (Some(request_id), Some(method), Some(response)) => Some((request_id, method, response)),
        _ => None,
    }))
}

fn request_id<T>(request: &Request<T>) -> Result<&str, Status> {
    let request_id = request
        .metadata()
        .get(REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    let valid = !request_id.is_empty()
        && request_id.len() <= MAX_REQUEST_ID_LEN
        && request_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-');
    if valid {
        Ok(request_id)
    } else {
        Err(Status::invalid_argument(format!(
            "{REQUEST_ID_HEADER} must be 1..={MAX_REQUEST_ID_LEN} alphanumeric characters"
        )))
    }
}
