---
title: "基盤とアプリの役割分担"
status: current
area: guide
related: [repo/operations, api/errors, api/common-response, server/transaction, server/auth, client/error-handling, client/auth-account, client/response-items]
proto: []
updated: 2026-10-11
---

# 基盤とアプリの役割分担

基盤(パッケージ)が持つものと、アプリが持つもの。アプリが作るのは「アプリ」の行のもので、基盤の行のものは作らない・変えない。

| もの | 置き場所 | 理由 |
|---|---|---|
| `player_id` の発行と、デバイスとの対応 | パッケージ(`devices.player_id`) | デバイス登録のときに発行する([認証](../api/auth.md))。プレイヤーの中身を知らなくても発行できる |
| `players` テーブル | アプリ | 持たせる列はアプリごとに違う。`player_id` を主キーにし、`device_id` は持たない。基盤(`voltech-grpc-auth`)は `player_id` からプレイヤーの状態(未作成・有効・停止)を引くtrait(`PlayerResolver`)だけを定義し、アプリが実装する |
| 直前のレスポンスを保存するテーブル([with_transaction() と冪等性](../server/transaction.md)) | パッケージ(`voltech-grpc-transaction`。マイグレーションを埋め込む) | `player_id` をキーにするだけで、`players` の中身には依存しない |
| `devices`・`access_tokens` テーブル([認証(サーバー)](../server/auth.md)) | パッケージ(マイグレーションを埋め込む) | 認証はパッケージの機能 |
| エラーの定義(`errors.toml`) | 予約エラーはパッケージ(domain `voltech`)、業務エラーはアプリ(アプリの domain) | エラーを domain と reason の組で見分けるので、基盤とアプリで別のファイルに書いてもぶつからない。アプリは予約エラーを変えられない([エラー](../api/errors.md)) |
| `PlayerDiff`・`BadgeType`・`Badge`・`Notification` の中身 | アプリ | 種類はアプリごとに違う。基盤の `.proto` はアプリの `.proto` を読み込めないので、基盤はフィールド名の規約(`player_diff`・`badges`・`notifications`)と、集めて返す・反映する仕組みだけを持つ([共通レスポンス項目](../api/common-response.md)) |
| クライアントの基盤(`ApiCore` など) | パッケージ | アプリは差し替え口だけを使う([運用(パッケージとサンプル)](../repo/operations.md)) |
| エラーダイアログ | パッケージはロジック(action → ボタン → 挙動)とインターフェース、アプリは見た目(prefab)と画面遷移 | 見た目はアプリごとに違う([エラー処理](../client/error-handling.md)) |
| カスタムエラーハンドラー・共通レスポンス項目の反映処理の実装 | アプリ | 何をするかはアプリごとに違う。基盤は呼び出す仕組みだけを持つ([エラー処理](../client/error-handling.md)、[共通レスポンス項目の反映](../client/response-items.md)) |
| アカウント情報の保存 | パッケージに既定の実装を置き、アプリが差し替えられる | 保存方法はアプリの要件次第([認証とアカウント情報](../client/auth-account.md)) |
