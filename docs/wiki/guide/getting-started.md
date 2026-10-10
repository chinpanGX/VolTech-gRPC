---
title: "始め方"
status: current
area: guide
related: [guide/add-api, ops/setup, repo/operations, api/errors]
proto: []
code: [sandbox/server/Cargo.toml, sandbox/server/proto/build.rs, sandbox/UnityProject/Packages/manifest.json, sandbox/errors.toml, buf.gen.sandbox.yaml, sandbox/server/server/src/lib.rs, sandbox/UnityProject/Assets/Scripts/Sample/SampleLifetimeScope.cs, .env.example]
updated: 2026-10-11
---

# 始め方

アプリは自分のプロジェクト(Unity プロジェクトと、Rust のサーバー)を作り、基盤(このリポジトリ、VolTech gRPC)をパッケージとして参照する。サーバーは Cargo の `git` 依存、クライアントは UPM の git URL で、どちらもタグを指定する。なぜこの形にしたかは [運用(パッケージとサンプル)](../repo/operations.md) にある。

組み込み方の実例は、このリポジトリの sandbox にある。基盤の機能を1つずつ試す RPC と、ガチャのサンプル(タイトル → サインアップ → ガチャ)が入っていて、手元で動かせる(下の「3. sandbox を動かす」)。

## 1. 基盤を参照する

サーバー(アプリの `Cargo.toml`)。クレート名で探されるので、ワークスペースの中の場所は書かなくてよい。

```toml
[workspace.dependencies]
voltech-grpc-auth = { git = "https://github.com/chinpanGX/VolTech-gRPC", tag = "v0.1.0" }
voltech-grpc-error = { git = "https://github.com/chinpanGX/VolTech-gRPC", tag = "v0.1.0" }
voltech-grpc-error-build = { git = "https://github.com/chinpanGX/VolTech-gRPC", tag = "v0.1.0" }
voltech-grpc-status = { git = "https://github.com/chinpanGX/VolTech-gRPC", tag = "v0.1.0" }
voltech-grpc-transaction = { git = "https://github.com/chinpanGX/VolTech-gRPC", tag = "v0.1.0" }
```

クライアント(Unity の `Packages/manifest.json`)。パッケージが依存する UniTask・YetAnotherHttpHandler(OpenUPM)と gRPC の DLL(Unity NuGet)を解決できるように、`scopedRegistries` も足す。

```json
{
  "dependencies": {
    "com.chinpangx.voltech.grpc": "https://github.com/chinpanGX/VolTech-gRPC.git?path=/Unity/com.chinpangx.voltech.grpc#v0.1.0"
  },
  "scopedRegistries": [
    { "name": "Unity NuGet", "url": "https://unitynuget-registry.openupm.com", "scopes": ["org.nuget"] },
    { "name": "OpenUPM", "url": "https://package.openupm.com", "scopes": ["com.cysharp.unitask", "com.cysharp.yetanotherhttphandler"] }
  ]
}
```

- タグは Cargo と UPM で同じ値を使う。まだタグを打っていないので、`v0.1.0` は打つ予定の名前(リリースの手順は [未確定・保留](../open-questions.md) #16)
- 基盤とアプリを同時に直すときは、手元のパスで差し替える([パッケージとアプリを同時に直すとき](../ops/local-override.md))

## 2. アプリの .proto とエラーを用意する

- アプリの `.proto` を書く([API を足す](add-api.md))。アプリの `.proto` は基盤の `.proto` を import しない
- アプリの業務エラーを、アプリの `errors.toml` に書く。domain はアプリの名前にする(`voltech` は使えない。[エラー](../api/errors.md))
- Rust は、アプリの `build.rs` で `.proto` と `errors.toml` から生成する(`voltech_grpc_error_build::compile`)。sandbox の `sandbox/server/proto/build.rs` が実例
- C# は、`buf generate` で `.proto` から(sandbox の設定は `buf.gen.sandbox.yaml`)、`protoc-gen-voltech-grpc errors` で `errors.toml` から生成し、生成したコードはコミットする。protoc プラグインは基盤のリポジトリの `tools/protoc-gen-voltech-grpc/` にある([ツール](tools.md))

## 3. sandbox を動かす

1. このリポジトリを clone し、`.env.example` を `.env` にコピーして、`make sandbox-server` でサーバーを起動する(MySQL を Docker で起動し、`localhost:50051` で待ち受ける)
2. Unity で `sandbox/UnityProject` を開き、`Assets/Scenes/Sample.unity` を Play する。Start を押すと、デバイス登録 → サインイン(初回はサインアップ)→ ガチャの画面に進む
3. `Window > VolTech gRPC > gRPC Monitor` で、通信ごとのヘッダーと Body を見られる([ツール](tools.md))

サーバーの状態(メンテナンス・強制アップデート・日付が変わる時刻)は `sandbox/server/config/server_status.toml` で決める。サーバーが 5 秒ごとに読み直すので、書き換えれば再起動しなくても効く([サーバーの状態](../server/status.md))。

必要なツールとコマンドの一覧は [環境構築とよく使うコマンド](../ops/setup.md) にある。

## 次に読むもの

- [API を足す](add-api.md)
- 組み込み方は [クライアントのリファレンス](client-reference.md)・[サーバーのリファレンス](server-reference.md)。sandbox での実例は、サーバーが `sandbox/server/server/src/lib.rs`、クライアントが `sandbox/UnityProject/Assets/Scripts/Sample/SampleLifetimeScope.cs`
