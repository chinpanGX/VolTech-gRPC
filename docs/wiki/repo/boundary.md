---
title: "基盤とアプリの境界"
status: deferred
area: repo
sections: ["2.5"]
related: [repo/operations, api/errors, api/common-response, server/transaction, server/auth, client/error-handling, client/auth-account, client/response-items]
proto: [sandbox/proto/sandbox/v1/error_code.proto]
updated: 2026-10-06
---

# 基盤とアプリの境界

| もの | 置き場所 | 理由 |
|---|---|---|
| `player_id` の発行と、デバイスとの対応 | パッケージ(`devices.player_id`) | デバイス登録のときに発行する([認証](../api/auth.md))。プレイヤーの中身を知らなくても発行できる |
| `players` テーブル | アプリ | 持たせる列はアプリごとに違う。`player_id` を主キーにし、`device_id` は持たない。パッケージ(当面は `server/rust/auth`)は `player_id` からプレイヤーの状態(未作成・有効・停止)を引くtrait(例: `PlayerResolver`)だけを定義し、アプリが実装する |
| 直前のレスポンスを保存するテーブル([4.3](../server/transaction.md)) | テンプレートの基盤 | `player_id` をキーにするだけで、`players` の中身には依存しない |
| `devices`・`access_tokens` テーブル([4.5](../server/auth.md)) | パッケージ(マイグレーションを埋め込む) | 認証はパッケージの機能 |
| `error_code.proto` | テンプレートに予約コード入りで置き、アプリが書き足す | enum は1ファイルにまとめる必要がある。1〜999は基盤の予約コード、1000以降をアプリが足す([3.5](../api/errors.md)) |
| `PlayerDiff`・`BadgeType`・`Badge`・`Notification` の中身 | アプリ | 種類はアプリごとに違う。基盤の `.proto` はアプリの `.proto` を読み込めないので、基盤はフィールド名の規約(`player_diff`・`badges`・`notifications`)と、集めて返す・反映する仕組みだけを持つ([3.9](../api/common-response.md)) |
| クライアントの基盤(`ApiCore` など) | パッケージ | アプリは差し替え口だけを使う([2.1](operations.md)) |
| エラーダイアログ | パッケージはロジック(action → ボタン → 挙動)とインターフェース、アプリは見た目(prefab)と画面遷移 | 見た目はアプリごとに違う([5.4](../client/error-handling.md)) |
| カスタムエラーハンドラー・共通レスポンス項目の反映処理の実装 | アプリ | 何をするかはアプリごとに違う。基盤は呼び出す仕組みだけを持つ([5.4](../client/error-handling.md)、[5.6](../client/response-items.md)) |
| アカウント情報の保存 | パッケージに既定の実装を置き、アプリが差し替えられる | 保存方法はアプリの要件次第([5.5](../client/auth-account.md)) |
