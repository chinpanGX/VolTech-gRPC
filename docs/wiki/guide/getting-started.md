---
title: "始め方"
status: current
area: guide
related: [guide/add-api, ops/setup, repo/operations]
proto: []
code: [sandbox/server/Cargo.toml, sandbox/UnityProject/Packages/manifest.json, sandbox/server/server/src/lib.rs, sandbox/UnityProject/Assets/Tests/EditMode/ApiServerTestBase.cs, sandbox/UnityProject/Assets/Tests/EditMode/SandboxServerTests.cs, buf.yaml, .env.example]
updated: 2026-10-10
---

# 始め方

アプリは、このリポジトリ(VolTech gRPC)をテンプレートとして clone して作る。基盤(`server/rust/`・`Unity/com.chinpangx.voltech.grpc/`・`tools/`)は clone したリポジトリの中にあり、`sandbox/` がアプリの雛形になる。雛形は基盤を手元のパスで参照する。なぜこの形にしたかは [運用(パッケージとテンプレート)](../repo/operations.md) にある。

ガチャを作り込んだ例は [grpc-foundation-samples](https://github.com/chinpanGX/grpc-foundation-samples) にある(このテンプレートから作ったアプリ)。

## 1. clone する

```
git clone https://github.com/chinpanGX/VolTech-gRPC my-app
cd my-app
git remote rename origin upstream
git remote add origin <アプリのリポジトリ>
```

- 基盤の改善は `git fetch upstream` → `git merge upstream/main` で取り込む
- **基盤のフォルダ(`server/rust/`・`Unity/com.chinpangx.voltech.grpc/`・`tools/`)はアプリで直さない**。直すと upstream からの取り込みで衝突する。足りない差し替え口があれば、基盤(upstream)に足す
- GitHub の「Use this template」は使わない(複製元との履歴のつながりが無くなり、upstream から取り込めない)

## 2. 雛形が基盤を参照する形

雛形の `sandbox/` は、基盤を手元のパスで参照している。アプリでもこのまま使う。

サーバー(`sandbox/server/Cargo.toml`。基盤とは別の Cargo ワークスペース):

```toml
[workspace.dependencies]
voltech-grpc-auth = { path = "../../server/rust/auth" }
voltech-grpc-error = { path = "../../server/rust/error" }
voltech-grpc-status = { path = "../../server/rust/status" }
voltech-grpc-transaction = { path = "../../server/rust/transaction" }
```

クライアント(`sandbox/UnityProject/Packages/manifest.json`)。パッケージが依存する UniTask・YetAnotherHttpHandler(OpenUPM)と gRPC の DLL(Unity NuGet)を解決するための `scopedRegistries` も入っている。

```json
{
  "dependencies": {
    "com.chinpangx.voltech.grpc": "file:../../../Unity/com.chinpangx.voltech.grpc"
  }
}
```

- アプリの `error_code.proto` は、基盤の `voltech/v1/error_options.proto` を手元のパスで import する(`buf.yaml` の `modules`、Rust は `sandbox/server/proto/build.rs`)
- `sandbox/` のフォルダ名と、雛形に入っている確認用の RPC(`Ping`・`Fail`・`Increment`)の扱いは未確定([未確定の論点](../open-questions.md) #23)

## 3. 動かす

1. `.env.example` を `.env` にコピーし、`make sandbox-server` でサーバーを起動する(MySQL を Docker で起動し、`localhost:50051` で待ち受ける)
2. Unity で `sandbox/UnityProject` を開く
3. Test Runner の EditMode で `SandboxServerTests` を実行すると、起動したサーバーにつないで、デバイス登録 → サインアップ → RPC の呼び出しまでを確かめられる
4. `Window > VolTech gRPC > Request Viewer` で、通信ごとのヘッダーと Body を見られる([ツール](tools.md))

サーバーの状態(メンテナンス・強制アップデート・日付が変わる時刻)は `sandbox/server/config/server_status.toml` で決める。サーバーが 5 秒ごとに読み直すので、書き換えれば再起動しなくても効く([サーバーの状態](../server/status.md))。

必要なツールとコマンドの一覧は [環境構築とよく使うコマンド](../ops/setup.md) にある。

## 次に読むもの

- [API を足す](add-api.md)
- 組み込み方は [クライアントのリファレンス](client-reference.md)・[サーバーのリファレンス](server-reference.md)。雛形での実例は `sandbox/server/server/src/lib.rs`・`sandbox/UnityProject/Assets/Tests/EditMode/ApiServerTestBase.cs`
