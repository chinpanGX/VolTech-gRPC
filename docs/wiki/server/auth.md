---
title: "認証(サーバー)"
status: confirmed
area: server
sections: ["4.5"]
source: "docs/feature-grpc-foundation.md"
related: [repo/boundary, api/auth, api/device-service]
proto: [server/rust/auth/proto/voltech/v1/device.proto]
code: [server/rust/auth]
updated: 2026-10-10
---

# 認証(サーバー)

パッケージの `voltech-grpc-auth` クレート(当面は `server/rust/auth`)が持つ。アプリとのつなぎ目は `PlayerResolver` の trait だけ。クライアントから呼ぶ API の仕様は [API: DeviceService](../api/device-service.md) にある。

- テーブル(`devices`・`access_tokens`)のマイグレーションはクレートに埋め込み(sqlx の `migrate!()`)、アプリは起動時に実行する
- **デバイス登録**: `secret_key` は Argon2 でハッシュ化して保存する(平文は保存しない)
- **トークン**: ランダムな文字列(Atlasは ULID を2つ連結)。`device_id` と有効期限と一緒にDBに保存する。有効期限は1時間、1デバイスにつき1つ(発行し直すと上書き)
- **`player_id`**: デバイス登録で ULID を発行し、`devices.player_id`(`BINARY(16)`)に保存する。デバイスとプレイヤーの対応はここだけで持ち、`players` は `device_id` を持たない([基盤とアプリの境界](../repo/boundary.md))
- **検証**: `AuthLayer` がメタデータ `authorization` を検証する。BAN・アカウント停止もここで判定する
- トークンは有効だがプレイヤーがまだいない状態(サインアップの前)を、`PlayerResolver`([2.5](../repo/boundary.md))が扱えるようにする
- **サーバーがプレイヤーを特定する根拠は、トークンだけ**。`device_id`・`player_id` はクライアントから受け取らない(秘密の値ではないので、信じるとなりすましができる)
- **`skip_auth` は v1 では読まず、ルーターで要認証のサービスにだけ `AuthLayer` を付ける**。付け分けのずれは結合テストで確かめる([6章](../ops/index.md))。ずれても `UNAUTHENTICATED` になってすぐ気づける(冪等性のずれのように、エラーが出ないまま二重実行になることは無い)

## アプリから使う

| 公開しているもの | 内容 |
|---|---|
| `migrate(&pool)` | テーブル(`devices`・`access_tokens`)を作成・更新する。起動時に呼ぶ。履歴は `_auth_migrations` に入る |
| `DeviceServiceImpl::new(pool).into_server()` | `DeviceService` をルーターに登録できる形にする。トークンの有効期限は `with_token_ttl` で変えられる(既定は1時間) |
| `AuthLayer::new(pool, resolver)` | 認証が必要なサービスに付ける Layer |
| `PlayerResolver` | `player_id` からプレイヤーの状態(`NotCreated`・`Active`・`Suspended`)を返す trait。アプリが実装する。`PlayerId` は ULID(`ulid::Ulid`) |
| `AuthContext` | 認証を通ったリクエストの `extensions()` に入る。`device_id`、`player_id`(サインアップの前からある)、`player_created`(サインアップ済みか) |
| `FILE_DESCRIPTOR_SET` | gRPC リフレクションに登録する FileDescriptorSet |

## テーブル

| テーブル | 内容 |
|---|---|
| `devices` | `device_id`、`player_id`、`secret_key` のハッシュ |
| `access_tokens` | デバイスごとに1行。トークンと有効期限 |

## 実装で決めたこと

- `AuthLayer` は `access_tokens` と `devices` を結合して、トークンから `device_id` と `player_id` を1回の問い合わせで引く
- `PlayerResolver::resolve` は `player_id` を受け取り、`PlayerStatus`(`NotCreated` / `Active` / `Suspended`)を返す。`Suspended` なら `AuthLayer` が `ACCOUNT_SUSPENDED` で弾く
- `AuthContext` は `player_id` を必ず持ち、サインアップ済みかを `player_created` で持つ。作成済みのプレイヤーが要るハンドラーは、未作成なら `PLAYER_NOT_FOUND` を返す(アプリの `api::context::player_id`)。サインアップだけは、作成済みかを問わずに `player_id` を使う
- 既存のデバイスには `player_id` を決められないので、マイグレーション(`20261004000003_add_player_id_to_devices.sql`)で開発・テストのデータを消してから列を足した。アプリ側も `20261004100005_remove_device_id_from_players.sql` で `players` から `device_id` を外した

## 採らなかった案

| 項目 | 採らなかった案 | 理由 |
|---|---|---|
| 認証の情報の渡し方 | クライアントが `device_id`・`player_id` をメタデータで送る | 秘密の値ではないので、信じるとなりすましができる。確かめるならトークンから引いた値を使えば足りる。仲介役のサービスがトークンを検証してヘッダーに入れる構成なら成り立つが、v1 は `AuthLayer` がその役を担い、ヘッダーではなく `AuthContext` で渡す |
| `player_id` の引き方 | `access_tokens` にも `player_id` を持たせる | 引き継ぎで対応が変わったときに、2か所を書き換えることになる。`devices` との結合で1回の問い合わせに収まる |
