---
title: "コードの地図"
status: current
area: appendix
related: [repo/operations, appendix/workflow, guide/client-reference, guide/server-reference]
proto: []
code: [server/rust/Cargo.toml, server/rust, sandbox/server, Unity/com.chinpangx.voltech.grpc, tools/protoc-gen-voltech-grpc, sandbox/proto, scripts, mk, buf.yaml, buf.gen.yaml]
updated: 2026-10-17
---

# コードの地図

基盤を直す人向けに、どこに何があるかをまとめる。

## 基盤(アプリごとに変えない)

| 置き場所 | 名前 | 中身 | テスト |
|---|---|---|---|
| `server/rust/auth/` | `voltech-grpc-auth` | `device_service.rs`(デバイス登録・トークンの発行)、`layer.rs`(`AuthLayer`)、`resolver.rs`(`PlayerResolver`)、`token.rs`(アクセストークンの値の生成)、`migrations/`、`proto/voltech/v1/`(`device.proto`・`auth_options.proto`) | `tests/`(MySQL を使う) |
| `server/rust/error/` | `voltech-grpc-error` | `lib.rs`(`ErrorCode` とエラーのレスポンスの作り方)、`internal.rs`(`Internal`・`ResultExt`)、`errors.toml`(基盤の予約エラーの定義。`build.rs` で `reserved` を生成する) | `tests/` |
| `server/rust/error-build/` | `voltech-grpc-error-build` | `errors.toml` の検証と、Rust の定数の生成(アプリと基盤の `build.rs` から使う) | `tests/` |
| `server/rust/log/` | `voltech-grpc-log` | `config.rs`(`LogConfig`)、`subscriber.rs`(`init`・`layer`、フィルター)、`json.rs`(JSON の1行の書き出し、ゲームログの名前付きの値の展開)、`rpc.rs`(`RpcLogLayer`・アクセスログ)、`method.rs`(`RpcMethod`)、`lib.rs`(span の項目名、`x-request-id` の形の確認) | `tests/` |
| `server/rust/status/` | `voltech-grpc-status` | `config.rs`(`server_status.toml` の読み込みと検証)、`status.rs`(`ServerStatus`・時計)、`source.rs`(読み込み元)、`layer.rs`(`ServerStatusLayer`・`DateChangeLayer`) | `tests/` |
| `server/rust/transaction/` | `voltech-grpc-transaction` | `lib.rs`(`Transactions::run`)、`context.rs`(`TxContext`)、`game_log.rs`(`GameLog`・`GameLogs`・`GameLogSink`、出力先への書き込み)、`idempotency.rs`(`idempotency_level` の表)、`method.rs`(`RpcMethodLayer`)、`migrations/`(`last_responses`・`game_logs`) | `tests/`(MySQL を使う) |
| `Unity/com.chinpangx.voltech.grpc/Runtime/Client/` | `VolTech.Client` | `ApiCore.cs`(共通処理)、`Errors/`(エラーの変換、`ErrorCode`、`ErrorPolicyProvider`、生成した予約エラー `ReservedErrors`)、`ErrorHandling/`(エラーハンドラー)、`Hooks/`(アプリが実装する interface)、`Clock/`(サーバー時刻)、`Testing/`(`FakeCallInvoker`) | `Tests/EditMode/`、sandbox の `ApiCoreTests` など |
| `Unity/com.chinpangx.voltech.grpc/Runtime/Auth/` | `VolTech.Client.Auth` | 認証(`DeviceTokenProvider`)とアカウントの保存先 | `Tests/EditMode/DeviceTokenProviderTests.cs` |
| `Unity/com.chinpangx.voltech.grpc/Runtime/Unity/` | `VolTech.Client.Unity` | Unity 用の層(`ApiChannel`・`UnityApiLogger` など) | sandbox の結合テスト |
| `Unity/com.chinpangx.voltech.grpc/Runtime/Generated/` | `VolTech.V1` | 基盤の `.proto`(`voltech.v1`)から生成した C#(コミットする) | — |
| `Unity/com.chinpangx.voltech.grpc/Runtime/Tracing/` | `VolTech.Client.Tracing` | 通信の記録(Interceptor・記録の置き場所・実機からの送信)。エディタと開発ビルドだけ | sandbox の `ApiTraceTests.cs` |
| `Unity/com.chinpangx.voltech.grpc/Editor/` | `VolTech.Client.Editor` | gRPC Monitor のウィンドウと、実機の記録の受け取り。エディタだけ | 目で確かめる |
| `tools/protoc-gen-voltech-grpc/` | `protoc-gen-voltech-grpc` | ラッパーを生成する protoc プラグイン([protoc プラグインの中身](protoc-plugin.md)) | `tests/` |

## sandbox(基盤の動作確認とサンプル。配らない)

基盤の機能を1つずつ試す RPC(`SandboxService`)と、組み込み方の実例のガチャのサンプルを置く。基盤は手元のパスで参照する。

| 置き場所 | 中身 |
|---|---|
| `sandbox/proto/sandbox/v1/` | `sandbox.proto`(`Ping`・`Fail`・`Increment`)、`player.proto`(`SignUp`・`SignIn`、`PlayerDiff`)、`gacha.proto`(`Draw`)、`badge.proto`・`notification.proto` |
| `sandbox/errors.toml` | sandbox の業務エラー(domain `sandbox`) |
| `sandbox/server/proto/` | sandbox の `.proto` と `errors.toml` から生成した Rust(`build.rs` で生成する) |
| `sandbox/server/player/`・`gacha/` | ドメインのクレート。`service/`(業務の本体)・`model/`、共通レスポンス項目のコレクター(`player` の `diff.rs`・`badge.rs`・`notification.rs`) |
| `sandbox/server/api/` | gRPC のハンドラー(`player.rs`・`gacha.rs`・`sandbox.rs`)と、コレクターからレスポンスへの変換 |
| `sandbox/server/server/` | バイナリ。Layer とサービスの組み立て(`lib.rs`)、`PlayerResolver` の実装(`player_lookup.rs`)。結合テストは `tests/` |
| `sandbox/server/migrations/`・`config/` | sandbox のテーブルのマイグレーション、`server_status.toml` |
| `sandbox/UnityProject/Assets/Generated/` | sandbox の `.proto` から生成した C#(コミットする) |
| `sandbox/UnityProject/Assets/Scripts/Sandbox/` | アプリのデータ(`PlayerRepository`・`BadgeStore`・`NotificationQueue`)と、共通レスポンス項目の反映処理 |
| `sandbox/UnityProject/Assets/Scripts/Sample/`・`Assets/Scenes/Sample.unity` | ガチャのサンプルの画面(タイトル・ガチャ・エラーダイアログ)と、VContainer の組み立て(`SampleLifetimeScope`) |
| `sandbox/UnityProject/Assets/Tests/EditMode/` | `ApiCore`・反映処理などのテストと、起動したサーバーへの結合テスト(`SandboxServerTests`・`PlayerApiConnectionTests`・`GachaApiConnectionTests`) |

## 依存の向き

- サーバー: `server/rust/` のクレートは sandbox やアプリに依存しない。`voltech-grpc-auth`・`-status`・`-transaction` は `voltech-grpc-error` に依存する。sandbox は別のワークスペースから `path` で基盤を参照する(アプリは `git` とタグで参照する。それ以外は同じ形)
- クライアント: `VolTech.Client` は Unity に依存しない(asmdef の No Engine References)。`VolTech.V1` を `VolTech.Client.Auth` が参照し(`VolTech.Client` は参照しない)、`VolTech.Client.Auth` を `VolTech.Client.Unity` が参照する。`VolTech.Client.Unity` は `VolTech.Client.Tracing` を参照しない(`Tracing` の側が起動時に `ApiChannel.AddInterceptor` で登録する。リリースビルドで消えるアセンブリを参照しないため)
- asmdef の参照は GUID で書く([構成](../client/structure.md))

## 設定と道具

| 置き場所 | 中身 |
|---|---|
| `buf.yaml`・`buf.gen.yaml`・`buf.gen.sandbox.yaml` | `.proto` のモジュール(基盤と sandbox)と lint の設定、C# の生成の設定(基盤 → パッケージ、sandbox → sandbox の Unity プロジェクト) |
| `Makefile` | 基盤の開発用のコマンド(`make help`)。アプリは使わない |
| `mk/env.mk` | 人や PC で変わる設定(`DOTNET`・`GIT_BASH`)の既定値。`local.mk`(コミットしない。例は `local.mk.example`)か環境変数で上書きする |
| `mk/codegen.mk` | C# の生成に使うツールとプラグインを入れるコマンド(`voltech-tools`・`proto-tools`・`proto-plugin`)と、生成の関数。基盤の `Makefile` と、アプリの Makefile が読む |
| `scripts/proto-tools.sh` | `.proto` の生成に使うツール(buf・protoc・grpc_csharp_plugin)を、引数のフォルダ(既定は `.tools/bin`)に入れる |
| `scripts/wiki_*.py` | `wiki_gen.py`(API の表の生成)、`wiki_lint.py`(wiki の確認)、`wiki_affected.py`(変わったファイルから直すべき wiki のページを探す) |
| `scripts/deps.py` | 依存の確認。`check`(一緒に上げないと壊れる組み合わせ。`make deps-check`・CI)、`outdated`(新しい版。`make deps-outdated`) |
| `docker-compose.yml`・`.env.example`・`docker/mysql/init/` | MySQL(基盤のテスト用の `app` と、sandbox の `voltech_sandbox`)と、サーバーの環境変数 |
