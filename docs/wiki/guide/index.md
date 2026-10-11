---
title: "アプリ開発者向けガイド"
status: index
area: guide
related: [overview, dev/index]
proto: []
updated: 2026-10-11
---

# アプリ開発者向けガイド

VolTech gRPC(基盤)を使ってアプリを作る人向けのページ。「何を・どう使うか」を書く。「なぜそうしたか」は設計のページにあり、各ページからリンクしている。

## 作る順に読む

1. [始め方](getting-started.md) — 基盤を参照し、C# の生成を準備する(sandbox で動くものを見る方法もここ)
2. [クライアントとサーバーをつなぐ](connect.md) — サーバーとクライアントを組み立て、つながるのを確かめる
3. [API を足す](add-api.md) — `.proto` を書き、生成し、ハンドラーと呼び出しとテストを書く。アプリの API はこれを繰り返して足す

## 必要なときに読む

- [基盤とアプリの役割分担](roles.md) — アプリが作るもの・作らないもの
- [通信を見る・調べる](debugging.md) — gRPC Monitor・grpcurl・Postman
- [基盤の版を上げる・基盤を一緒に直す](upgrade.md)
- [クライアントのリファレンス](client-reference.md) — `ApiCore`・ラッパー・基盤の実装・テスト用の部品
- [サーバーのリファレンス](server-reference.md) — `voltech-grpc-*` のクレートで使うもの
- API の仕様: [DeviceService(デバイス登録・トークンの発行)](../api/device-service.md)。アプリの API の仕様は、アプリの `.proto` のコメントが正
