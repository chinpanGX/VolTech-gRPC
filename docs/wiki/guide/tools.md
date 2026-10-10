---
title: "ツール"
status: current
area: guide
related: [api/codegen, client/request-viewer, ops/setup, ops/debugging]
proto: []
code: [tools/protoc-gen-voltech-grpc/src/GeneratorParameters.cs, buf.gen.yaml, buf.yaml, Makefile, Unity/com.chinpangx.voltech.grpc/Editor, Unity/com.chinpangx.voltech.grpc/Runtime/Tracing]
updated: 2026-10-10
---

# ツール

基盤が用意している開発用のツール。

## protoc プラグイン(`protoc-gen-voltech-grpc`)

`.proto` から、クライアントで使う C# を生成する。`buf generate` で、protoc 組み込みの C# 生成(メッセージ型)・`grpc_csharp_plugin`(gRPC クライアント)と一緒に動く。設定は `buf.gen.yaml` にある。

生成するもの(`.proto` のファイルごとに `<ファイル名>Api.cs` を1つ):

- **ラッパー**(サービスごとに1クラス。`GachaService` → `GachaApi`)。RPC ごとに `XxxAsync(request, handledCodes)` を持ち、`ApiCore` を通して呼ぶ。`.proto` の `idempotency_level`(冪等性の対象か)と `skip_auth`(認証が要るか)を読んで埋め込む
- **RPC のパスの定数**(`GachaServiceMethods.Draw = "/app.v1.GachaService/Draw"`)
- **共通レスポンス項目の目印**: `player_diff`・`badges`・`notifications` を持つレスポンスに `IHasPlayerDiff`・`IHasBadges`・`IHasNotifications` を付ける

オプション(`buf.gen.yaml` の `opt`):

| オプション | 中身 |
|---|---|
| `package=app.v1` | 生成する `.proto` のパッケージ。複数書ける。書かなければ全部を生成する。基盤の `voltech.v1` は `DeviceTokenProvider` が直接呼ぶので、ラッパーを作らないように絞っている |
| `error_code=app.v1.ErrorCode` | `handledCodes` に使う enum の完全名。書かなければ、名前が `ErrorCode` の enum を探す |

使い方は `make proto-gen`(プラグインをビルドしてから生成する)。プラグインだけをビルドするのは `make proto-plugin`(`.tools/voltech-grpc` に入る)。設計の理由は [コード生成](../api/codegen.md)、中身は [protoc プラグインの中身](../dev/protoc-plugin.md)。

## 通信の Viewer(Request Viewer)

Unity エディタで、API のリクエストとレスポンスを1件ずつ見るウィンドウ。メニュー `Window > VolTech gRPC > Request Viewer` で開く。

- 一覧から1件を選ぶと、リクエストとレスポンスのヘッダー・Body(JSON)・トレーラー・エラーの詳細(`ErrorInfo`)を見られる
- Body の「Copy」で JSON をコピーし、grpcurl や Postman にそのまま貼れる
- Toolbar の「Editor / Player」で、USB でつないだ実機(開発ビルド)の通信に切り替えられる
- 記録するのはエディタと開発ビルドだけ(リリースビルドにはアセンブリごと入らない)。アクセストークンなどがそのまま見えるため
- アプリは何もしなくてよい。`ApiChannel.CreateCallInvoker` で作った通信が記録される

設計と実装の詳細は [通信の Viewer](../client/request-viewer.md)。

## make のコマンド

リポジトリのルートで実行する。一覧は `make help`、よく使うものは [環境構築とよく使うコマンド](../ops/setup.md)。

| コマンド | 内容 |
|---|---|
| `make sandbox-server` | MySQL と sandbox のサーバーを起動する(samples では `make run`) |
| `make proto-gen` | `.proto` から C# を生成する |
| `make proto-lint` / `make proto-breaking` | `.proto` の lint / main との互換性の確認 |
| `make test` | プッシュ前のテスト一式 |
| `make client-lint` | クライアントの書き方の確認 |
| `make wiki-lint` / `make wiki-affected` | wiki の確認 / 変わったファイルから直すべき wiki のページを探す |
| `make deps-outdated` / `make deps-check` | 基盤の依存の新しい版を調べる / 一緒に上げないと壊れる組み合わせを確かめる |

## 外部のツール

- **Postman・grpcurl**: サーバーは開発環境(`APP_ENV=development`)で gRPC リフレクションを有効にするので、`.proto` を読み込まなくても呼べる([デバッグ](../ops/debugging.md))
