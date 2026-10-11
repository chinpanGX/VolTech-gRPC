---
title: "通信を見る・調べる"
status: current
area: guide
related: [client/grpc-monitor, guide/connect, api/metadata, api/errors]
proto: []
code: [Unity/com.chinpangx.voltech.grpc/Editor, Unity/com.chinpangx.voltech.grpc/Runtime/Tracing, Unity/com.chinpangx.voltech.grpc/Runtime/Unity/ApiChannel.cs]
updated: 2026-10-11
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
- サーバー: 業務エラー以外のエラー(`internal`)の詳細は、サーバーのログにだけ出る。クライアントには返らない([エラー](../api/errors.md))
