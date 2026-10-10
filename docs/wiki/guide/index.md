---
title: "利用者向けガイド"
status: index
area: guide
sections: []
related: [dev/index, overview]
proto: []
updated: 2026-10-10
---

# 利用者向けガイド

VolTech gRPC(基盤)を使ってアプリを作る人向けのページ。「何を・どう使うか」を書く。「なぜそうしたか」は設計のページ([概要](../overview.md) から辿れる)にあり、各ページからリンクしている。

基盤そのものを直す人は [基盤開発者向けガイド](../dev/index.md) を見る。

## ページ

- [始め方](getting-started.md) — テンプレート(このリポジトリ)を clone して、サーバーとクライアントを動かすまで
- [API を足す](add-api.md) — `.proto` を書き、コードを生成し、サーバーのハンドラーとクライアントの呼び出しを書くまで
- [クライアントのリファレンス](client-reference.md) — `ApiCore`・生成したラッパー・アプリが実装する interface・テスト用の部品
- [サーバーのリファレンス](server-reference.md) — `voltech-grpc-*` のクレートと、サーバーへの組み込み方
- [ツール](tools.md) — protoc プラグイン・通信の Viewer・`make` のコマンド
- API の仕様: [DeviceService(デバイス登録・トークンの発行)](../api/device-service.md)。アプリの API の仕様は、アプリの `.proto` のコメントが正
