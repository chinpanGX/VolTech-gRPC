---
title: "始め方"
status: current
area: guide
sections: []
related: [guide/add-api, ops/setup, repo/current-plan]
proto: []
code: [Makefile, .env.example, docker-compose.yml, Unity/UnityProject/Assets/Scripts/Sample/SampleLifetimeScope.cs]
updated: 2026-10-10
---

# 始め方

アプリは、このリポジトリ(サンプル兼テンプレート)を clone して作る。基盤(VolTech gRPC)は、今はこのリポジトリの中にあり、`voltech-grpc` に分けた後はパッケージとして参照する([当面の実装方針](../repo/current-plan.md))。

必要なツールとバージョンは [環境構築とよく使うコマンド](../ops/setup.md) にある。

## 1. サーバーを動かす

1. `.env.example` を `.env` にコピーする。DB の接続先(`DATABASE_URL`)・待ち受けるアドレス(`LISTEN_ADDR`)・サーバーの状態のファイル(`SERVER_STATUS_PATH`)と、`APP_ENV=development`(gRPC リフレクションを有効にする)が入っている
2. `make run` を実行する。MySQL を Docker で起動し、マイグレーションを流してから、サーバーを `localhost:50051` で起動する
3. ログに `listening addr=0.0.0.0:50051` が出たら動いている

サーバーの状態(メンテナンス・強制アップデート・日付が変わる時刻)は `Server/app/config/server_status.toml` で決める。サーバーが 5 秒ごとに読み直すので、書き換えれば再起動しなくても効く([サーバーの状態](../server/status.md))。

## 2. クライアントを動かす

1. Unity で `Unity/UnityProject` を開く
2. `Assets/Scenes/Sample.unity` を開き、Play する。タイトル画面のボタンを押すと、デバイス登録 → サインイン(初回はサインアップ)→ ガチャの画面に進む
3. `Window > VolTech gRPC > Request Viewer` で、通信ごとのヘッダーと Body を見られる([ツール](tools.md))

接続先は `SampleLifetimeScope` のインスペクターの `serverAddress`(既定は `http://localhost:50051`)で変える。実機から PC のサーバーにつなぐときは、`localhost` ではなく PC のアドレスにする。

## 3. テストを流す

- サーバー: `make test-rust`(MySQL を使う)
- クライアント: Unity の Test Runner の EditMode。サーバーにつなぐテスト(`ApiServerTestBase` を継ぐもの)は、`make run` でサーバーを起動してから流す
- まとめて: `make test`(Rust・protoc プラグイン・wiki の確認)

## 次に読むもの

- [API を足す](add-api.md)
- 組み込み方の実例は、クライアントは `SampleLifetimeScope`([クライアントのリファレンス](client-reference.md))、サーバーは `Server/app/server/src/lib.rs`([サーバーのリファレンス](server-reference.md))
