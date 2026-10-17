---
title: "サーバーのリファレンス"
status: current
area: guide
related: [guide/add-api, server/request-flow, server/transaction, server/game-log, server/logging, server/auth, server/status, api/errors]
proto: []
code: [server/rust/auth/src, server/rust/error/src, server/rust/log/src, server/rust/status/src, server/rust/transaction/src, sandbox/server/server/src/lib.rs, sandbox/server/server/src/main.rs, sandbox/server/server/src/config.rs, sandbox/server/server/src/player_lookup.rs, sandbox/server/api/src, sandbox/server/proto/build.rs]
updated: 2026-10-17
---

# サーバーのリファレンス

サーバー(Rust)で使う、基盤のクレートの一覧。各項目の細かい説明は、コードのドキュメントコメント(`///`)が正。ここでは、どのクレートの何を使うかと、組み込み方を書く。

| クレート | 置き場所 | 中身 |
|---|---|---|
| `voltech-grpc-auth` | `server/rust/auth/` | デバイス登録とトークンの発行(`DeviceService`)、認証の Layer、テーブルのマイグレーション |
| `voltech-grpc-error` | `server/rust/error/` | エラーのレスポンスの作り方、内部エラーの原因を運ぶ型(`Internal`)と、基盤が返す予約エラーコードの名前 |
| `voltech-grpc-log` | `server/rust/log/` | ログの出力の準備と、RPC ごとの span・アクセスログの Layer |
| `voltech-grpc-status` | `server/rust/status/` | メンテナンス・強制アップデート・日付変更の判定と、サーバー時刻・ゲーム内の日付のヘッダー |
| `voltech-grpc-transaction` | `server/rust/transaction/` | トランザクションと冪等性(同じ RequestID の再送で二重に実行しない)、ゲームログ、テーブルのマイグレーション |

`Cargo.toml` で使うクレートだけを依存に入れる。コードでは `voltech_grpc_auth::AuthLayer` のように書く。組み立て方は [クライアントとサーバーをつなぐ](connect.md)。

## アプリが実装するもの

| trait | 役割 | サンプルの実装 |
|---|---|---|
| `voltech_grpc_auth::PlayerResolver` | `player_id` からプレイヤーの状態(`PlayerStatus::NotCreated`・`Active`・`Suspended`)を返す。`players` テーブルを持つアプリが実装する。停止中なら `AuthLayer` が `ACCOUNT_SUSPENDED` を返す | sandbox の `PlayerLookup`(`sandbox/server/server/src/player_lookup.rs`) |
| `voltech_grpc_status::StatusSource` | サーバーの状態の設定の読み込み元。既定は `FileStatusSource`(TOML のファイル) | 基盤の `FileStatusSource` を使う |

## ハンドラーで使う

| 名前 | 使い方 |
|---|---|
| `voltech_grpc_transaction::Transactions::run(&request, player_id, \|ctx\| ...)` | ハンドラーの本体をトランザクションの中で動かす。成功したらコミット、失敗したらロールバックする。冪等性の対象の RPC では、同じ `x-request-id` の再送なら本体を呼ばずに保存済みのレスポンスを返す(`player_id` が要る)。`NO_SIDE_EFFECTS` は読み取り専用のトランザクションになる([with_transaction() と冪等性](../server/transaction.md)) |
| `voltech_grpc_transaction::TxContext` | `run` に渡す処理が受け取る。`ctx.tx()` でトランザクション(`&mut sqlx::Transaction`)を、`ctx.write_game_log(log)` でゲームログを書く。ゲームログは `NO_SIDE_EFFECTS` の RPC と、`AuthLayer` を付けていないサービスでは書けない(INTERNAL) |
| `voltech_grpc_transaction::GameLog` | ゲームログの1件。数量の変化は `GameLog::new(kind, delta, after, reason)`、数量の変化でないもの(キャラクターの入手・編成の変更など)は `GameLog::event(kind, reason)`。対象があれば `.target(id)`、名前付きの値は `.field(name, value)` を付ける。`player_id`・`request_id`・RPC・時刻は基盤が付ける([ゲームログ](../server/game-log.md)) |
| `voltech_grpc_transaction::GameLogs` / `GameLogSink::commit(tx, logs)` | `Transactions::run` を通らない処理(サーバー間の内部 API・定期実行のジョブ)でゲームログを書く。`GameLogs::new(処理の名前)` に `write(player_id, log)` でためて、`commit` で出力先へ書いてトランザクションをコミットする |
| `voltech_grpc_transaction::create_last_response(ctx.tx(), player_id)` | プレイヤーを作るときに、同じトランザクションで呼ぶ(冪等性のために直前のレスポンスを保存する行を作る) |
| `voltech_grpc_auth::AuthContext` | `AuthLayer` がリクエストの `extensions()` に入れる認証の情報。`device_id`・`player_id`・`player_created`(サインアップ済みか) |
| `voltech_grpc_status::GameDate` | `ServerStatusLayer` がリクエストの `extensions()` に入れる、リクエストを受けたときのゲーム内の日付。1つのリクエストの中では、この日付を使う |
| `voltech_grpc_error::failed_precondition(code, message)` | 業務エラー(`FAILED_PRECONDITION` + `ErrorInfo` の domain・reason)。`code` はアプリの `errors.toml` から生成した定数(例: `proto::errors::INSUFFICIENT_GEMS`) |
| `voltech_grpc_error::failed_precondition_with_metadata(code, message, metadata)` | 表示用の値(`ErrorInfo.metadata`)付きの業務エラー。キー `action` は使わない |
| `voltech_grpc_error::Internal` | 内部エラーの原因(元のエラー・処理の説明・発生した場所)を運ぶ型。業務の本体の関数は `Result<_, Internal>` を返し、`std::error::Error` を実装したエラーを `?` で包む。`Status` には `?`・`.into()` でなり、そのとき INTERNAL にして原因の連なりをログに出す。業務エラーの `Status` を包んでいたら、元のまま返す。元のエラーが無いときは `Internal::msg("...")`、`Box<dyn Error>` は `Internal::from_boxed` |
| `voltech_grpc_error::internal(err)` | `Internal` か `std::error::Error` を実装したエラーから、サーバー内部エラーの `Status` を作る。詳細はログにだけ出し、クライアントには返さない |
| `voltech_grpc_error::ResultExt` | `result.or_internal()`(エラーを INTERNAL の `Status` に)と `result.context("...")`(説明を付けた `Internal` に)。`.map_err(internal)` と関数を渡すと発生した場所が残らないので、代わりに `or_internal()` を使う |
| `voltech_grpc_error::ErrorCode` / `reserved::*` | エラーコード(domain と reason)と、基盤の予約エラー(domain `voltech`。`MAINTENANCE`・`INTERNAL_ERROR` など)。テストで返ったエラーを確かめるときに使う |
| `voltech_grpc_error_build::compile(path)` | アプリの `build.rs` から呼び、アプリの `errors.toml` からエラーコードの定数を生成する(sandbox は `sandbox/server/proto/build.rs`。[エラー](../api/errors.md)) |

sandbox では、`AuthContext` と `GameDate` を取り出す関数を `sandbox/server/api/src/context.rs` にまとめている(付け忘れの Layer があれば INTERNAL にする)。

## Layer と設定

| 名前 | 中身 |
|---|---|
| `voltech_grpc_status::ServerStatusLayer` | 強制アップデート(`x-app-platform`・`x-app-version` を最低バージョンと比べる)とメンテナンスを判定し、レスポンスに `x-server-time`・`x-game-date` を付ける。DB を使わない |
| `voltech_grpc_auth::AuthLayer` | `authorization: Bearer <token>` を検証し、`AuthContext` を入れる。認証が要るサービスにだけ付ける |
| `voltech_grpc_status::DateChangeLayer` | リクエストの `x-game-date` が今のゲーム内の日付と違えば `DATE_CHANGED` を返す。`AuthLayer` の内側に付ける |
| `voltech_grpc_log::RpcLogLayer` | RPC ごとに span を作り(`request_id`・`rpc.method` などがその中のログに付く)、RPC の終わりにアクセスログを1行出す。ハンドラーの panic は INTERNAL にする。全サービスの一番外側に付ける([運用ログ](../server/logging.md)) |
| `voltech_grpc_transaction::RpcMethodLayer` | RPC の名前をリクエストに入れる(`Transactions::run` が冪等性の扱いを引くため)。全サービスの外側に付ける |
| `voltech_grpc_status::StatusConfig` | `server_status.toml` の中身(`[maintenance]`・`[min_version]`・`[game_day]`)。知らないキーや範囲外の値はエラーにする([サーバーの状態](../server/status.md)) |
| `voltech_grpc_auth::DeviceServiceImpl` | `DeviceService` の実装。トークンの有効期限は 1 時間(`with_token_ttl` で変えられる) |
| `voltech_grpc_auth::FILE_DESCRIPTOR_SET` | 基盤の `.proto` の FileDescriptorSet。gRPC リフレクションに登録する |
| `voltech_grpc_status` のヘッダー名の定数 | `APP_PLATFORM_HEADER`・`APP_VERSION_HEADER`・`GAME_DATE_HEADER`・`SERVER_TIME_HEADER`・`MAINTENANCE_END_AT_KEY`([メタデータ](../api/metadata.md)) |

## ログ

起動時に `voltech_grpc_log::init(&config)` を1回呼び、`Server::builder()` の一番外側に `RpcLogLayer::new(&config)` を付ける。何がどう出るかは [運用ログ](../server/logging.md)。

| 名前 | 中身 |
|---|---|
| `voltech_grpc_log::LogConfig` | ログの設定。`LogConfig::new(service_name, environment)` で既定の値を入れてから、変えたい項目(`format`・`filter`・`field_names`・`excluded_rpcs`)を書き換える |
| `voltech_grpc_log::init(&config)` | 標準出力へ出す準備をし、プロセス全体の出力先にする。フィルターの書き間違いはエラー |
| `voltech_grpc_log::layer(&config)` | `init` と同じ出力の Layer。OpenTelemetry などのほかの Layer と組み合わせるときに、`tracing_subscriber::registry().with(...)` に足す |
| `voltech_grpc_log::LogFormat` / `FieldNames` | 出力の形式(`json`・`text`)と JSON の項目名(`standard`・`gcp`)。`"json".parse()` のように文字列から読め、知らない値はエラー |
| `voltech_grpc_transaction::GameLogSink` | ゲームログの出力先。`Database`(`game_logs` テーブル)か `Stdout`(標準出力の JSON)。`LogConfig` ではなく `Transactions::new` に渡す |

基盤は環境変数を読まない。アプリが読んで `LogConfig` に入れる。環境変数の名前は次をおすすめする(sandbox の `sandbox/server/server/src/config.rs` が読み方の見本)。

| 環境変数 | `LogConfig` の項目 | 値 |
|---|---|---|
| `LOG_FORMAT` | `format` | `json`(本番)・`text`(手元) |
| `RUST_LOG` | `filter` | 例: `info`、`warn,app=debug` |
| `LOG_FIELD_NAMES` | `field_names` | `standard`・`gcp`(Cloud Logging に送るとき) |
| `OTEL_SERVICE_NAME` | `service_name` | サーバーの名前。OpenTelemetry と同じ名前の環境変数にする |
| `APP_ENV` | `environment` | 例: `development`・`staging`・`production` |

ゲームログの出力先も、アプリが読んで `Transactions::new` に渡す。

| 環境変数 | 渡す先 | 値 |
|---|---|---|
| `GAME_LOG_SINK` | `Transactions::new` の `GameLogSink` | `database`(`Database`)・`stdout`(`Stdout`。ログの収集・分析のサービスへ送る構成) |
