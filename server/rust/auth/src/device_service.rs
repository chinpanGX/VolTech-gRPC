use std::ops::RangeInclusive;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use argon2::Argon2;
use argon2::password_hash::phc::PasswordHash;
use argon2::password_hash::{PasswordHasher, PasswordVerifier};
use sqlx::MySqlPool;
use tonic::{Request, Response, Status};
use ulid::Ulid;

use crate::proto::voltech::v1::device_service_server::{DeviceService, DeviceServiceServer};
use crate::proto::voltech::v1::{
    IssueTokenRequest, IssueTokenResponse, RegisterRequest, RegisterResponse,
};
use crate::token;
use voltech_grpc_error::internal;

const DEFAULT_TOKEN_TTL: Duration = Duration::from_secs(60 * 60);
const SECRET_KEY_LEN: RangeInclusive<usize> = 16..=128;

/// `DeviceService`(デバイス登録とトークンの発行)の実装。
#[derive(Clone)]
pub struct DeviceServiceImpl {
    pool: MySqlPool,
    token_ttl: Duration,
}

impl DeviceServiceImpl {
    /// トークンの有効期限を 1 時間にしてサービスを作る。
    pub fn new(pool: MySqlPool) -> Self {
        Self {
            pool,
            token_ttl: DEFAULT_TOKEN_TTL,
        }
    }

    /// 発行するトークンの有効期限を変える。
    pub fn with_token_ttl(mut self, token_ttl: Duration) -> Self {
        self.token_ttl = token_ttl;
        self
    }

    /// tonic のルーターに登録できる形にする。
    pub fn into_server(self) -> DeviceServiceServer<Self> {
        DeviceServiceServer::new(self)
    }
}

#[tonic::async_trait]
impl DeviceService for DeviceServiceImpl {
    async fn register(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        let secret_key = request.into_inner().secret_key;
        validate_secret_key(&secret_key)?;

        let secret_hash = hash_secret(secret_key).await?;
        let device_id = Ulid::generate().to_string();
        // ID だけを発行し、プレイヤー(初期データ)はアプリのサインアップで作る
        let player_id = Ulid::generate();
        sqlx::query("INSERT INTO devices (device_id, player_id, secret_hash) VALUES (?, ?, ?)")
            .bind(&device_id)
            .bind(player_id.to_bytes().as_slice())
            .bind(&secret_hash)
            .execute(&self.pool)
            .await
            .map_err(internal)?;

        Ok(Response::new(RegisterResponse {
            device_id,
            player_id: player_id.to_string(),
        }))
    }

    async fn issue_token(
        &self,
        request: Request<IssueTokenRequest>,
    ) -> Result<Response<IssueTokenResponse>, Status> {
        let IssueTokenRequest {
            device_id,
            secret_key,
        } = request.into_inner();

        let secret_hash: Option<String> =
            sqlx::query_scalar("SELECT secret_hash FROM devices WHERE device_id = ?")
                .bind(&device_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(internal)?;
        let Some(secret_hash) = secret_hash else {
            return Err(invalid_credentials());
        };
        if !verify_secret(secret_key, secret_hash).await? {
            return Err(invalid_credentials());
        }

        let access_token = token::generate();
        let expires_at = (SystemTime::now() + self.token_ttl)
            .duration_since(UNIX_EPOCH)
            .map_err(internal)?
            .as_secs() as i64;
        sqlx::query(
            "INSERT INTO access_tokens (device_id, token, expires_at) \
             VALUES (?, ?, FROM_UNIXTIME(?)) AS new \
             ON DUPLICATE KEY UPDATE token = new.token, expires_at = new.expires_at",
        )
        .bind(&device_id)
        .bind(&access_token)
        .bind(expires_at)
        .execute(&self.pool)
        .await
        .map_err(internal)?;

        Ok(Response::new(IssueTokenResponse {
            access_token,
            expires_at: Some(prost_types::Timestamp {
                seconds: expires_at,
                nanos: 0,
            }),
        }))
    }
}

fn validate_secret_key(secret_key: &str) -> Result<(), Status> {
    // ASCII だけにして、文字数とバイト数が食い違わないようにする
    let printable_ascii = secret_key.bytes().all(|b| b.is_ascii_graphic());
    if SECRET_KEY_LEN.contains(&secret_key.len()) && printable_ascii {
        Ok(())
    } else {
        Err(Status::invalid_argument(format!(
            "secret_key must be {}..={} printable ASCII characters (no spaces)",
            SECRET_KEY_LEN.start(),
            SECRET_KEY_LEN.end()
        )))
    }
}

fn invalid_credentials() -> Status {
    Status::unauthenticated("invalid device credentials")
}

// Argon2 は意図的に重い計算なので、非同期のワーカーを塞がないように別スレッドで行う
async fn hash_secret(secret_key: String) -> Result<String, Status> {
    tokio::task::spawn_blocking(move || {
        Argon2::default()
            .hash_password(secret_key.as_bytes())
            .map(|hash| hash.to_string())
    })
    .await
    .map_err(internal)?
    .map_err(internal)
}

async fn verify_secret(secret_key: String, secret_hash: String) -> Result<bool, Status> {
    tokio::task::spawn_blocking(move || {
        let parsed = PasswordHash::new(&secret_hash)?;
        Ok::<_, argon2::password_hash::Error>(
            Argon2::default()
                .verify_password(secret_key.as_bytes(), &parsed)
                .is_ok(),
        )
    })
    .await
    .map_err(internal)?
    .map_err(internal)
}
