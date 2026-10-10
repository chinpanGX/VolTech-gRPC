---
title: "デバッグ"
status: confirmed
area: ops
sections: ["6.2"]
related: [client/api-core]
proto: []
updated: 2026-10-07
---

# デバッグ

- **Postman** を使う。サーバーは開発環境のときだけ gRPC リフレクションを有効にする(`tonic-reflection`)ので、`.proto` を変えるたびにインポートし直さなくてよい
- RequestIDや認証トークンは、Postman の Metadata タブで設定して送る
- 手順をコマンドで残したいときは grpcurl も使える。サーバーを作る前に応答の形を確かめたいときは、Postman のモックサーバーも使える
- Unity エディタでは、[通信の Viewer](../client/request-viewer.md) で、通信ごとにヘッダーと Body(JSON)を見られる。Body はコピーして、Postman や grpcurl にそのまま貼り付けられる。USB でつないだ実機(開発ビルド)の通信も見られる
- Unity の EditMode テストのうちサーバーにつなぐもの(`ApiServerTestBase` を継ぐもの)は、`ApiCore` のロガー(`UnityApiLogger`)でリクエストとレスポンスを Console に出す。`ApiCore` ができるまで使っていたテスト専用の Interceptor(`LoggingInterceptor`)は消した
