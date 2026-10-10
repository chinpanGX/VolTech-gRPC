---
title: "付録: Atlasの乗せ換え"
status: appendix
area: appendix
related: [overview, api/errors, server/collectors, client/wrapper, client/api-core, client/error-handling, client/auth-account, client/response-items, client/fake-testing]
proto: []
updated: 2026-10-10
---

# 付録: Atlasの乗せ換え

Atlasを乗せ換える([概要](../overview.md))ときのメモ。テンプレートの設計には関わらない。

### 置き換わるもの

| Atlasの今の仕組み | gRPCでの置き換え先 |
|---|---|
| api-codegen が生成する `ApiRequest`(UnityWebRequestでJSONを送受信) | `GrpcChannel` + YetAnotherHttpHandler、生成されたメッセージ型 |
| api-codegen が生成する `XxxApiClient` | 生成されたクライアントと、[ラッパー](../client/wrapper.md) |
| `Authorization` ヘッダー、ログ出力 | `ApiCore`([ApiCore の共通処理](../client/api-core.md)) |
| `ApiException`(HTTPステータスと本文) | エラーコードと action、`ApiResult`([エラー処理](../client/error-handling.md)) |
| `player_id` をサインアップで発行し、`players.device_id` でデバイスと対応させる | デバイス登録で発行し、`devices.player_id` で対応させる([認証](../api/auth.md))。乗せ換えでは、既存の `players` の `device_id` から `devices.player_id` を埋めるデータ移行が要る |
| HTTPステータス5種類のエラー(`AppError`)と、クライアントのステータスでの分岐(`401` で再認証、`404` でサインアップへ) | 業務ごとのエラーコード([エラー](../api/errors.md))。「プレイヤー未作成」は `handledCodes` で受け取る |
| `AccessTokenRefresher`(期限前の再認証、`401` 時の再送、single-flight) | `ApiCore`([認証とアカウント情報](../client/auth-account.md)) |
| `PlayerDiffApplier`、サーバーでの差分の手組み(`build_full_player_diff` など) | 反映処理([共通レスポンス項目の反映](../client/response-items.md))、差分[コレクター](../server/collectors.md) |

- BattleServer(C#)からの内部API(`/internal/battle/*`)は、乗せ換えの段階ではRESTのまま残す(移すかは乗せ換え時に判断する)
- Atlasのスカウトはテンプレートのサンプルにしない。パチモン・技・バナーなどのマスタに依存し、基盤と関係ない仕組みまでテンプレートに入ってしまうため

### クライアントの今の構成の課題

今の呼び出し経路: `Presenter → XxxService → IXxxConnection → XxxApiClient → ApiRequest`

| # | 課題 | 基盤での扱い |
|---|---|---|
| 1 | Serviceが素通りになっている(1回の通信に4層を経由する) | アプリの設計として、サンプルを作るときに決める |
| 2 | Repositoryの更新が見えない場所(Connection内)で起きる | 「playerDiffは必ず `ApiCore` が反映する」にまとめる([共通レスポンス項目の反映](../client/response-items.md)) |
| 3 | 共通処理(トークン更新、playerDiffの反映、DTOの詰め替え)を毎回手で書いている | `ApiCore` と生成したラッパーで解消する([ラッパー](../client/wrapper.md)、[ApiCore の共通処理](../client/api-core.md)) |
| 4 | Connectionの分け方がサーバーのURLに引きずられている | アプリの設計として、サンプルを作るときに決める |
| 5 | Mock/Realの方針と実装が食い違っている | `FakeCallInvoker` で切り替えをDIの登録1か所にまとめる([サーバーが無いときの差し替えとテスト](../client/fake-testing.md)) |
| 6 | 生成クライアントをConnectionの中で生成しており、通信部分だけを差し替えられない | アプリの設計として、サンプルを作るときに決める |
