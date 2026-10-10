mod device_service;
mod layer;
mod resolver;
mod token;

pub use device_service::DeviceServiceImpl;
pub use layer::{AuthContext, AuthLayer, AuthService};
pub use resolver::{BoxError, PlayerId, PlayerResolver, PlayerStatus};

pub mod proto {
    pub mod voltech {
        pub mod v1 {
            tonic::include_proto!("voltech.v1");
        }
    }
}

/// `device.proto` とその依存の FileDescriptorSet。gRPC リフレクションの登録に使う。
pub const FILE_DESCRIPTOR_SET: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/auth_descriptor.bin"));

/// 認証のテーブル(`devices`・`access_tokens`)を作成・更新する。アプリの起動時に呼ぶ。
pub async fn migrate(pool: &sqlx::MySqlPool) -> Result<(), sqlx::migrate::MigrateError> {
    let mut migrator = sqlx::migrate!("./migrations");
    // アプリのマイグレーションと履歴を同じテーブルに入れると、互いに相手の分を
    // 「知らないマイグレーションが適用済み」とみなしてエラーになるので分ける
    migrator.dangerous_set_table_name("_auth_migrations");
    migrator.run(pool).await
}
