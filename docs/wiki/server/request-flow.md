---
title: "リクエストの処理の流れ"
status: confirmed
area: server
sections: ["4.2"]
related: [api/metadata, server/transaction, server/auth, server/status]
proto: []
code: [sandbox/server/server/src/lib.rs]
updated: 2026-10-10
---

# リクエストの処理の流れ

| 順 | 処理 | 実装 | 返すもの |
|---|---|---|---|
| 1 | ログ | `tower_http::TraceLayer` などの汎用Layer | |
| 2 | 強制アップデートの判定 | `voltech-grpc-status` クレートの `ServerStatusLayer`([4.6](status.md)。DBを使わない) | `UPDATE_REQUIRED` |
| 3 | メンテナンスの判定 | `voltech-grpc-status` クレートの `ServerStatusLayer`([4.6](status.md)。DBを使わない) | `MAINTENANCE` |
| 4 | 認証 | `voltech-grpc-auth` クレートの `AuthLayer`(非同期の `tower::Service`。[4.5](auth.md))。`skip_auth` 以外のサービスにだけルーターで付ける | `UNAUTHENTICATED`・`ACCOUNT_SUSPENDED` |
| 5 | 日付変更の判定 | `voltech-grpc-status` クレートの `DateChangeLayer`([4.6](status.md)。DBを使わない)。`AuthLayer` の内側に付ける | `DATE_CHANGED` |
| 6 | ハンドラー | `with_transaction()` を呼ぶ([4.3](transaction.md)) | 成功、または業務エラー |

- 強制アップデートをメンテナンスより先に判定する。メンテナンスと同時に新しいバージョンを出すとき、古いアプリのユーザーにメンテナンス中のうちにアップデートを促せる
- 2・3・5はDBを使わないので、メンテナンスでDBを止めていても判定できる
- 認証は、トークンの検証にDBへの問い合わせが要るので、非同期の `tower::Service` で書く(`tonic::Interceptor` は同期関数専用)
- 冪等性のチェック・トランザクション・コレクターは、ミドルウェアにせず、ハンドラーが `with_transaction()` を明示的に呼ぶ。これらはレスポンスの型に依存する処理で、tower Layer で汎用的に書くと、gRPCの成否がHTTP/2のtrailerに乗ることや、レスポンスのFuture型が統一できないことなどの制約にぶつかる。また、sqlx の `Transaction` は `Sync` ではないので、`http::Extensions` に載せて渡せない
- エラーの変換(`AppError` → `tonic::Status`)は、`with_transaction()` やハンドラーの戻り値の変換で行う
- 成功したレスポンスには、サーバー時刻と今のゲーム内の日付をヘッダーで付ける([3.8](../api/metadata.md))。付けるのは `ServerStatusLayer` で、エラーのレスポンスにも付くが、クライアントは成功したときだけ読む
- **タイムアウト**: tonic は、クライアントの deadline(`grpc-timeout`)とサーバー側のタイムアウトのうち短い方で処理を打ち切る。打ち切られたトランザクションは sqlx が自動でロールバックするので、再送しても安全

## 採らなかった案

| 項目 | 採らなかった案 | 理由 |
|---|---|---|
| サーバーの共通処理 | 冪等性・トランザクション・コレクターを tower Layer にする | レスポンスの型に依存し、trailer やFuture型の制約にぶつかる。`Transaction` が `Sync` でなく Extensions に載らない |
| サーバーの共通処理 | 認証を `tonic::Interceptor` で書く | 同期関数専用で、DBに問い合わせられない |
