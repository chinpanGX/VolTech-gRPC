---
title: "サーバーのリファレンス"
status: current
area: guide
related: [guide/add-api, server/request-flow, server/transaction, server/auth, server/status, api/errors]
proto: []
code: [server/rust/auth/src, server/rust/error/src, server/rust/status/src, server/rust/transaction/src, sandbox/server/server/src/lib.rs, sandbox/server/server/src/main.rs, sandbox/server/server/src/player_lookup.rs, sandbox/server/api/src, sandbox/server/proto/build.rs]
updated: 2026-10-11
---

# サーバーのリファレンス

サーバー(Rust)で使う、基盤のクレートの一覧。各項目の細かい説明は、コードのドキュメントコメント(`///`)が正。ここでは、どのクレートの何を使うかと、組み込み方を書く。

| クレート | 置き場所 | 中身 |
|---|---|---|
| `voltech-grpc-auth` | `server/rust/auth/` | デバイス登録とトークンの発行(`DeviceService`)、認証の Layer、テーブルのマイグレーション |
| `voltech-grpc-error` | `server/rust/error/` | エラーのレスポンスの作り方と、基盤が返す予約エラーコードの名前 |
| `voltech-grpc-status` | `server/rust/status/` | メンテナンス・強制アップデート・日付変更の判定と、サーバー時刻・ゲーム内の日付のヘッダー |
| `voltech-grpc-transaction` | `server/rust/transaction/` | トランザクションと冪等性(同じ RequestID の再送で二重に実行しない)、テーブルのマイグレーション |

`Cargo.toml` で使うクレートだけを依存に入れる。コードでは `voltech_grpc_auth::AuthLayer` のように書く。

## 組み込み方

sandbox の `sandbox/server/server/src/lib.rs` が実例。Layer の順番に意味がある([リクエストの処理の流れ](../server/request-flow.md))。

```rust
// 起動時: 基盤とアプリのマイグレーション
voltech_grpc_auth::migrate(&pool).await?;
voltech_grpc_transaction::migrate(&pool).await?;
sqlx::migrate!("../migrations").run(&pool).await?;

// 冪等性の扱いは、アプリの .proto の idempotency_level から決める
let transactions = Transactions::new(
    pool.clone(),
    IdempotencyLevels::from_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)?,
);
let auth = AuthLayer::new(pool.clone(), PlayerLookup::new(pool.clone())); // PlayerLookup はアプリの PlayerResolver
let date_change = DateChangeLayer::new(status.clone());

Server::builder()
    .layer(ServerStatusLayer::new(status))   // 強制アップデート・メンテナンス(全サービス。認証より先)
    .layer(RpcMethodLayer)                   // Transactions::run が RPC の名前を読めるように
    .add_service(DeviceServiceImpl::new(pool.clone()).into_server())        // 認証なし
    .add_service(auth.layer(date_change.layer(GachaServiceImpl::new(transactions).into_server())))
    // ...
```

サーバーの状態は、起動時に `ServerStatus::watch(FileStatusSource::new(path), interval)` で読み込み、以後 `interval` ごとに読み直す(sandbox は 5 秒。`sandbox/server/server/src/main.rs`)。

## アプリが実装するもの

| trait | 役割 | サンプルの実装 |
|---|---|---|
| `voltech_grpc_auth::PlayerResolver` | `player_id` からプレイヤーの状態(`PlayerStatus::NotCreated`・`Active`・`Suspended`)を返す。`players` テーブルを持つアプリが実装する。停止中なら `AuthLayer` が `ACCOUNT_SUSPENDED` を返す | sandbox の `PlayerLookup`(`sandbox/server/server/src/player_lookup.rs`) |
| `voltech_grpc_status::StatusSource` | サーバーの状態の設定の読み込み元。既定は `FileStatusSource`(TOML のファイル) | 基盤の `FileStatusSource` を使う |

## ハンドラーで使う

| 名前 | 使い方 |
|---|---|
| `voltech_grpc_transaction::Transactions::run(&request, player_id, \|tx\| ...)` | ハンドラーの本体をトランザクションの中で動かす。成功したらコミット、失敗したらロールバックする。冪等性の対象の RPC では、同じ `x-request-id` の再送なら本体を呼ばずに保存済みのレスポンスを返す(`player_id` が要る)。`NO_SIDE_EFFECTS` は読み取り専用のトランザクションになる([with_transaction() と冪等性](../server/transaction.md)) |
| `voltech_grpc_transaction::create_last_response(tx, player_id)` | プレイヤーを作るときに、同じトランザクションで呼ぶ(冪等性のために直前のレスポンスを保存する行を作る) |
| `voltech_grpc_auth::AuthContext` | `AuthLayer` がリクエストの `extensions()` に入れる認証の情報。`device_id`・`player_id`・`player_created`(サインアップ済みか) |
| `voltech_grpc_status::GameDate` | `ServerStatusLayer` がリクエストの `extensions()` に入れる、リクエストを受けたときのゲーム内の日付。1つのリクエストの中では、この日付を使う |
| `voltech_grpc_error::failed_precondition(code, message)` | 業務エラー(`FAILED_PRECONDITION` + `ErrorInfo` の domain・reason)。`code` はアプリの `errors.toml` から生成した定数(例: `proto::errors::INSUFFICIENT_GEMS`) |
| `voltech_grpc_error::failed_precondition_with_metadata(code, message, metadata)` | 表示用の値(`ErrorInfo.metadata`)付きの業務エラー。キー `action` は使わない |
| `voltech_grpc_error::internal(err)` | サーバー内部エラー。詳細はログにだけ出し、クライアントには返さない |
| `voltech_grpc_error::ErrorCode` / `reserved::*` | エラーコード(domain と reason)と、基盤の予約エラー(domain `voltech`。`MAINTENANCE`・`INTERNAL_ERROR` など)。テストで返ったエラーを確かめるときに使う |
| `voltech_grpc_error_build::compile(path)` | アプリの `build.rs` から呼び、アプリの `errors.toml` からエラーコードの定数を生成する(sandbox は `sandbox/server/proto/build.rs`。[エラー](../api/errors.md)) |

sandbox では、`AuthContext` と `GameDate` を取り出す関数を `sandbox/server/api/src/context.rs` にまとめている(付け忘れの Layer があれば INTERNAL にする)。

## Layer と設定

| 名前 | 中身 |
|---|---|
| `voltech_grpc_status::ServerStatusLayer` | 強制アップデート(`x-app-platform`・`x-app-version` を最低バージョンと比べる)とメンテナンスを判定し、レスポンスに `x-server-time`・`x-game-date` を付ける。DB を使わない |
| `voltech_grpc_auth::AuthLayer` | `authorization: Bearer <token>` を検証し、`AuthContext` を入れる。認証が要るサービスにだけ付ける |
| `voltech_grpc_status::DateChangeLayer` | リクエストの `x-game-date` が今のゲーム内の日付と違えば `DATE_CHANGED` を返す。`AuthLayer` の内側に付ける |
| `voltech_grpc_transaction::RpcMethodLayer` | RPC の名前をリクエストに入れる(`Transactions::run` が冪等性の扱いを引くため)。全サービスの外側に付ける |
| `voltech_grpc_status::StatusConfig` | `server_status.toml` の中身(`[maintenance]`・`[min_version]`・`[game_day]`)。知らないキーや範囲外の値はエラーにする([サーバーの状態](../server/status.md)) |
| `voltech_grpc_auth::DeviceServiceImpl` | `DeviceService` の実装。トークンの有効期限は 1 時間(`with_token_ttl` で変えられる) |
| `voltech_grpc_auth::FILE_DESCRIPTOR_SET` | 基盤の `.proto` の FileDescriptorSet。gRPC リフレクションに登録する |
| `voltech_grpc_status` のヘッダー名の定数 | `APP_PLATFORM_HEADER`・`APP_VERSION_HEADER`・`GAME_DATE_HEADER`・`SERVER_TIME_HEADER`・`MAINTENANCE_END_AT_KEY`([メタデータ](../api/metadata.md)) |
