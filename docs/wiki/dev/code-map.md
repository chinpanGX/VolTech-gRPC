---
title: "コードの地図"
status: current
area: dev
sections: []
related: [repo/current-plan, dev/workflow, guide/client-reference, guide/server-reference]
proto: []
code: [Server/Cargo.toml, Server/framework, Server/app, Unity/UnityProject/Assets/Scripts, Tools/protoc-gen-voltech-grpc, Shared/proto, Shared/scripts, buf.yaml, buf.gen.yaml]
updated: 2026-10-10
---

# コードの地図

基盤を直す人向けに、どこに何があるかをまとめる。フォルダの分け方の方針は [当面の実装方針](../repo/current-plan.md)。

## 基盤(アプリごとに変えない)

| 置き場所 | 名前 | 中身 | テスト |
|---|---|---|---|
| `Server/framework/auth/` | `voltech-grpc-auth` | `device_service.rs`(デバイス登録・トークンの発行)、`layer.rs`(`AuthLayer`)、`resolver.rs`(`PlayerResolver`)、`token.rs`(アクセストークンの値の生成)、`migrations/`、`proto/voltech/v1/`(`device.proto`・`auth_options.proto`) | `tests/`(MySQL を使う) |
| `Server/framework/error/` | `voltech-grpc-error` | `lib.rs`(エラーのレスポンスの作り方)、`code.rs`(予約エラーコードの名前)、`proto/voltech/v1/error_options.proto` | — |
| `Server/framework/status/` | `voltech-grpc-status` | `config.rs`(`server_status.toml` の読み込みと検証)、`status.rs`(`ServerStatus`・時計)、`source.rs`(読み込み元)、`layer.rs`(`ServerStatusLayer`・`DateChangeLayer`) | `tests/` |
| `Server/framework/transaction/` | `voltech-grpc-transaction` | `lib.rs`(`Transactions::run`)、`idempotency.rs`(`idempotency_level` の表)、`method.rs`(`RpcMethodLayer`)、`migrations/` | `tests/`(MySQL を使う) |
| `Unity/UnityProject/Assets/Scripts/VolTech.Grpc.Client/` | `VolTech.Grpc.Client` | `ApiCore.cs`(共通処理)、`Errors/`(エラーの変換)、`ErrorHandling/`(エラーハンドラー)、`Hooks/`(アプリが実装する interface)、`Clock/`(サーバー時刻)、`Testing/`(`FakeCallInvoker`) | `Tests/EditMode/` |
| `Unity/UnityProject/Assets/Scripts/VolTech.Grpc.Client.Tracing/` | `VolTech.Grpc.Client.Tracing` | 通信の記録(Interceptor・記録の置き場所・実機からの送信)。エディタと開発ビルドだけ | `Tests/EditMode/ApiTraceTests.cs` |
| `Unity/UnityProject/Assets/Scripts/VolTech.Grpc.Client.Editor/` | `VolTech.Grpc.Client.Editor` | 通信の Viewer のウィンドウと、実機の記録の受け取り。エディタだけ | 目で確かめる |
| `Unity/UnityProject/Assets/Scripts/Api/Auth/`・`Unity/`・`Errors/` | `Api`(の一部) | 認証(`DeviceTokenProvider`)、Unity 用の層(`ApiChannel` など)、エラーコードの表(`ProtoErrorPolicyProvider`)。アプリのコードと同じアセンブリにあり、基盤を分けるときに分ける | `Tests/EditMode/` |
| `Tools/protoc-gen-voltech-grpc/` | `protoc-gen-voltech-grpc` | ラッパーを生成する protoc プラグイン([protoc プラグインの中身](protoc-plugin.md)) | `tests/` |

## アプリ(サンプル。アプリごとに変える)

| 置き場所 | 中身 |
|---|---|
| `Shared/proto/app/v1/` | アプリの `.proto`(`player.proto`・`gacha.proto`・`error_code.proto` など) |
| `Server/app/proto/` | アプリの `.proto` から生成した Rust(`build.rs` で生成する) |
| `Server/app/player/`・`gacha/` | ドメインのクレート(service → model) |
| `Server/app/api/` | gRPC のハンドラー |
| `Server/app/server/` | バイナリ。Layer とサービスの組み立て(`lib.rs`)、設定(`config.rs`)、`PlayerResolver` の実装(`player_lookup.rs`)。結合テストは `tests/` |
| `Server/app/migrations/`・`config/` | アプリのテーブルのマイグレーション、`server_status.toml` |
| `Unity/UnityProject/Assets/Scripts/Api/Generated/` | `.proto` から生成した C#(コミットする) |
| `Unity/UnityProject/Assets/Scripts/Api/Repositories/`・`Badges/`・`Notifications/` | アプリの反映処理と、データの置き場所 |
| `Unity/UnityProject/Assets/Scripts/Sample/` | サンプルの画面と、組み立て(`SampleLifetimeScope`) |

## 依存の向き

- サーバー: `framework/` のクレートは `app/` に依存しない。`voltech-grpc-auth`・`-status`・`-transaction` は `voltech-grpc-error` に依存する
- クライアント: `VolTech.Grpc.Client` は Unity に依存しない(asmdef の No Engine References)。`Api` は `VolTech.Grpc.Client` を参照し、`VolTech.Grpc.Client.Tracing` を参照しない(`Tracing` の側が起動時に `ApiChannel.AddInterceptor` で登録する。リリースビルドで消えるアセンブリを参照しないため)
- asmdef の参照は GUID で書く([構成](../client/structure.md))

## 設定と道具

| 置き場所 | 中身 |
|---|---|
| `buf.yaml`・`buf.gen.yaml` | `.proto` のモジュール(アプリと基盤)と lint の設定、C# の生成の設定 |
| `Makefile` | 開発用のコマンド(`make help`) |
| `scripts/proto-tools.sh` | `.proto` の生成に使うツール(buf・protoc・grpc_csharp_plugin)を `.tools/bin` に入れる |
| `Shared/scripts/` | `wiki_gen.py`(API の表の生成)、`wiki_lint.py`(wiki の確認)、`wiki_affected.py`(変わったファイルから直すべき wiki のページを探す) |
| `docker-compose.yml`・`.env.example` | MySQL と、サーバーの環境変数 |
