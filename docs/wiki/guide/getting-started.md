---
title: "始め方"
status: current
area: guide
sections: []
related: [guide/add-api, ops/setup, ops/local-override]
proto: []
code: [sandbox/server/server/src/lib.rs, sandbox/UnityProject/Assets/Tests/EditMode/ApiServerTestBase.cs]
updated: 2026-10-10
---

# 始め方

アプリは [grpc-foundation-samples](https://github.com/chinpanGX/grpc-foundation-samples)(サンプル兼テンプレート)を clone して作る。基盤(このリポジトリ、VolTech gRPC)は、サーバーは Cargo の `git` 依存、クライアントは UPM の git URL で参照する。

基盤そのものを直すときの環境は [環境構築とよく使うコマンド](../ops/setup.md) にある。

## 1. 基盤を参照する

サーバー(アプリの `Cargo.toml`)。クレート名で探されるので、ワークスペースの中の場所は書かなくてよい。

```toml
[workspace.dependencies]
voltech-grpc-auth = { git = "https://github.com/chinpanGX/VolTech-gRPC", tag = "v0.1.0" }
voltech-grpc-error = { git = "https://github.com/chinpanGX/VolTech-gRPC", tag = "v0.1.0" }
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

- タグは Cargo と UPM で同じ値を使う
- アプリの `error_code.proto` は、基盤の `voltech/v1/error_options.proto` を import する。基盤の `.proto` をアプリからどう参照するか(コピーするか、`buf` の依存にするか)は未確定([未確定の論点](../open-questions.md))
- 基盤とアプリを同時に直すときは、手元のパスで差し替える([パッケージとテンプレートを同時に直すとき](../ops/local-override.md))

## 2. samples を動かす

手順の詳細は samples の wiki にある。流れだけ書く。

1. `.env.example` を `.env` にコピーし、`make run` でサーバーを起動する(MySQL を Docker で起動し、`localhost:50051` で待ち受ける)
2. Unity で samples の Unity プロジェクトを開き、サンプルのシーンを Play する。デバイス登録 → サインイン(初回はサインアップ)→ ガチャの画面に進む
3. `Window > VolTech gRPC > Request Viewer` で、通信ごとのヘッダーと Body を見られる([ツール](tools.md))

サーバーの状態(メンテナンス・強制アップデート・日付が変わる時刻)は `server_status.toml` で決める。サーバーが 5 秒ごとに読み直すので、書き換えれば再起動しなくても効く([サーバーの状態](../server/status.md))。

## 次に読むもの

- [API を足す](add-api.md)
- 組み込み方は [クライアントのリファレンス](client-reference.md)・[サーバーのリファレンス](server-reference.md)。基盤だけで組み立てた最小の例は、このリポジトリの sandbox(`sandbox/server/server/src/lib.rs`・`sandbox/UnityProject/Assets/Tests/EditMode/ApiServerTestBase.cs`)
