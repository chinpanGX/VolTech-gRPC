---
title: "通信を見る・調べる"
status: current
area: guide
related: [client/grpc-monitor, guide/connect, api/metadata, api/errors, server/logging]
proto: []
code: [Unity/com.chinpangx.voltech.grpc/Editor, Unity/com.chinpangx.voltech.grpc/Runtime/Tracing, Unity/com.chinpangx.voltech.grpc/Runtime/Unity/ApiChannel.cs, server/rust/log/src]
updated: 2026-10-16
---

# 通信を見る・調べる

クライアントが何を送り、サーバーが何を返したかを確かめる道具。

## gRPC Monitor(Unity エディタ)

Unity エディタで、API のリクエストとレスポンスを1件ずつ見るウィンドウ。メニュー `Window > VolTech gRPC > gRPC Monitor` で開く。

- 一覧から1件を選ぶと、リクエストとレスポンスのヘッダー・Body(JSON)・トレーラー・エラーの詳細(`ErrorInfo`)を見られる
- Body の「Copy」で JSON をコピーし、grpcurl や Postman にそのまま貼れる
- Toolbar の「Editor / Player」で、USB でつないだ実機(開発ビルド)の通信に切り替えられる
- 記録するのはエディタと開発ビルドだけ(リリースビルドにはアセンブリごと入らない)。アクセストークンなどがそのまま見えるため
- アプリは何もしなくてよい。`ApiChannel.CreateCallInvoker` で作った通信が記録される
- 同じ RequestID(`x-request-id`)の行が並んでいれば、再送した通信

設計と実装の詳細は [gRPC Monitor](../client/grpc-monitor.md)。

## Postman・grpcurl(サーバーを直接呼ぶ)

- サーバーで gRPC リフレクションを有効にしておくと([クライアントとサーバーをつなぐ](connect.md))、`.proto` を読み込まなくても呼べる。`.proto` を変えるたびにインポートし直さなくてよい
- 認証が要る RPC は、トークン(`authorization: Bearer <token>`)と RequestID(`x-request-id`)などのヘッダーを付けて送る。Postman なら Metadata タブで設定する([メタデータ](../api/metadata.md))。トークンは、gRPC Monitor で `IssueToken` のレスポンスから写せる
- 手順をコマンドで残したいときは grpcurl を使う

```
grpcurl -plaintext localhost:50051 list
grpcurl -plaintext -H "authorization: Bearer <token>" -H "x-request-id: <毎回違う文字列>" -d '{}' localhost:50051 app.v1.GachaService/Draw
```

- サーバーを作る前に応答の形を確かめたいときは、Postman のモックサーバーも使える。クライアントだけで試すなら `FakeCallInvoker` もある([サーバーが無いときの差し替えとテスト](../client/fake-testing.md))

## ログ

- クライアント: `ApiCore` に `UnityApiLogger` を渡すと、通信のログを Console に出す(開発中だけ渡す)
- サーバー: 業務エラー以外のエラー(INTERNAL)の詳細は、サーバーのログにだけ出る。クライアントには返らない([エラー](../api/errors.md))。ログはメッセージ `internal error` の行で、原因の連なり(`exception.message`)と発生した場所(`code.file.path`・`code.line.number`)が付く
- 認証やトークンの発行で失敗したときは、`authentication failed`・`access token issue failed` の行の `reason` で理由が分かる(期限切れか、知らないトークンかなど)。基盤が出すログの一覧は [運用ログ](../server/logging.md)
- 所持データ(ジェム・アイテムなど)がいつ・どの RPC で変わったかは、ゲームログで調べる。DB に書く設定なら `game_logs` テーブルを `player_id` で引く([ゲームログ](../server/game-log.md))
- サーバーのログは、RPC ごとに `request_id`(`x-request-id`)と `rpc.method` が付く。gRPC Monitor で見た RequestID でサーバーのログを探すと、その通信の中で出たログと、終わりのアクセスログ(`rpc finished`。ステータス・業務エラーの reason・処理時間)が見つかる([運用ログ](../server/logging.md))
- 手元では `LOG_FORMAT=text` にすると、JSON ではなく読みやすいテキストで出る。細かく見たいときは `RUST_LOG=debug` などにする([サーバーのリファレンス](server-reference.md) のログの節)
