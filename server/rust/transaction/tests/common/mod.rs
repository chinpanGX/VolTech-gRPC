//! テストで使う DB の接続と、RPC の名前・ヘッダーを入れたリクエストの作り方。

use prost::Message;
use prost_types::method_options::IdempotencyLevel;
use prost_types::{
    FileDescriptorProto, FileDescriptorSet, MethodDescriptorProto, MethodOptions,
    ServiceDescriptorProto,
};
use sqlx::MySqlPool;
use tonic::Request;
use voltech_grpc_transaction::{IdempotencyLevels, RpcMethod, Transactions};

const DEFAULT_DATABASE_URL: &str = "mysql://app:app@localhost:13306/app";

/// テスト用のサービスの名前。
pub const SERVICE: &str = "test.v1.SampleService";
/// 冪等性の対象の RPC(idempotency_level を書いていない)。
pub const DRAW: &str = "Draw";
/// 冪等性の対象の、もう1つの RPC。
pub const BUY: &str = "Buy";
/// `NO_SIDE_EFFECTS` の RPC。
pub const READ: &str = "Read";
/// `IDEMPOTENT` の RPC。
pub const SIGN_IN: &str = "SignIn";

/// マイグレーションを実行し、テスト用のサービスの表を持った `Transactions` を作る。
pub async fn start() -> (Transactions, MySqlPool) {
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_owned());
    let pool = MySqlPool::connect(&database_url).await.unwrap();
    voltech_grpc_transaction::migrate(&pool).await.unwrap();

    let levels = IdempotencyLevels::from_file_descriptor_set(&sample_descriptor_set()).unwrap();
    (Transactions::new(pool.clone(), levels), pool)
}

/// RPC の名前と、`x-request-id`(`None` なら付けない)を入れたリクエストを作る。
pub fn request(method: &'static str, request_id: Option<&str>) -> Request<()> {
    let mut request = Request::new(());
    request
        .extensions_mut()
        .insert(RpcMethod::from_path(&format!("/{SERVICE}/{method}")).unwrap());
    if let Some(request_id) = request_id {
        request
            .metadata_mut()
            .insert("x-request-id", request_id.parse().unwrap());
    }
    request
}

fn sample_descriptor_set() -> Vec<u8> {
    let method = |name: &str, level: Option<IdempotencyLevel>| MethodDescriptorProto {
        name: Some(name.to_owned()),
        options: level.map(|level| MethodOptions {
            idempotency_level: Some(level as i32),
            ..Default::default()
        }),
        ..Default::default()
    };
    FileDescriptorSet {
        file: vec![FileDescriptorProto {
            package: Some("test.v1".to_owned()),
            service: vec![ServiceDescriptorProto {
                name: Some("SampleService".to_owned()),
                method: vec![
                    method(DRAW, None),
                    method(BUY, None),
                    method(READ, Some(IdempotencyLevel::NoSideEffects)),
                    method(SIGN_IN, Some(IdempotencyLevel::Idempotent)),
                ],
                ..Default::default()
            }],
            ..Default::default()
        }],
    }
    .encode_to_vec()
}
