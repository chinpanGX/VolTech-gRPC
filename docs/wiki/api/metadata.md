---
title: "メタデータ(ヘッダー)"
status: confirmed
area: api
sections: ["3.8"]
source: "docs/feature-grpc-foundation.md"
related: [api/idempotency, api/auth, server/status, client/api-core]
proto: []
updated: 2026-10-10
---

# メタデータ(ヘッダー)

| 向き | 内容 | 用途 |
|---|---|---|
| リクエスト | `authorization: Bearer <token>`(`skip_auth` のサービスには付けない) | 認証([3.7](auth.md)) |
| リクエスト | `x-request-id` | 冪等性([3.6](idempotency.md))。対象のRPCだけ |
| リクエスト | `x-app-platform: ios \| android \| windows \| editor` と `x-app-version: <アプリのバージョン>` | 強制アップデートの判定([4.6](../server/status.md))。デバイス登録・トークン発行を含む全リクエストに付ける。バージョンは表示用の「1.2.0」ではなく、リリースのたびに増やす整数(iOS の Build、Android の versionCode と同じ値) |
| リクエスト | `x-game-date: YYYY-MM-DD`(セッションの最初に受け取ったもの) | 日付変更の判定([4.6](../server/status.md))。まだ受け取っていない、セッションの最初の呼び出し(サインアップ・サインイン)とトークン発行には付かない |
| レスポンス | `x-server-time: <Unix 時間のミリ秒>` | クライアントのサーバー時刻の同期([5.3](../client/api-core.md)) |
| レスポンス | `x-game-date: YYYY-MM-DD`(今のゲーム内の日付) | 日付変更の判定([4.6](../server/status.md)) |

- 名前は、サーバーは `voltech-grpc-status` クレートの定数(`APP_PLATFORM_HEADER` など)、クライアントは `ApiHeaders` にまとめている。両方を一緒に直す
- ゲーム内の日付は、リクエストとレスポンスで同じキーにした。どちらも「どの日付のセッションか」を表すため
- 独自のヘッダーには `x-` を付け、`x-request-id` とそろえる。gRPC の決まり(小文字の英数字と `-`・`_`・`.` だけ、`grpc-` で始まる名前は予約、`-bin` で終わる名前はバイナリの値)は満たしている。名前は最初のリリースまでに確定させる(リリース後は古いアプリが古い名前を送り続ける)

## 採らなかった案

| 項目 | 採らなかった案 | 理由 |
|---|---|---|
| ヘッダーの名前 | `x-client-platform` / `x-client-build` | 何を表すかが分かりにくい。送っているのはアプリのプラットフォームとバージョンなので、`x-app-` にそろえた |
| ヘッダーの名前 | `x-` を付けない(RFC 6648 の勧め) | gRPC に名前の付け方の規則は無く、`x-` でも決まりは満たしている。確定済みの `x-request-id` とそろえるほうが読みやすい |
| ヘッダーの名前 | 基盤の名前を付ける(Google の `x-goog-*` のような `x-<名前>-`) | パッケージ名([2.2](../repo/naming.md))がまだ決まっていない。名前がぶつかる心配も今は無い |
