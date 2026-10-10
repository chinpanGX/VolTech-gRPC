---
title: "API定義"
status: index
area: api
sections: ["3"]
updated: 2026-10-10
---

# API定義

> 原本の見出し: 3. API定義 【確定】

`.proto` で決める、サーバーとクライアントの約束ごと。実装はサーバー([4章](../server/index.md))とクライアント([5章](../client/index.md))に書く。

## ページ

- [基本規約(型の定義元・通信方式・nullable と oneof)](conventions.md) — マスターと API の型の分離、v1 は Unary のみ、nullable / oneof の規約
- [コード生成](codegen.md) — コード生成の流れ、自作 protoc プラグイン、カスタムオプション
- [エラー](errors.md) — ErrorInfo による返し方、エラーコードと action、予約コード
- [冪等性(RequestID と idempotency_level)](idempotency.md) — 再送しても一度だけ処理される仕組みと、RPC ごとの宣言
- [認証](auth.md) — デバイス認証(ゲスト型)の流れと skip_auth
- [メタデータ(ヘッダー)](metadata.md) — リクエスト/レスポンスのメタデータ一覧
- [共通レスポンス項目](common-response.md) — player_diff・badges・notifications の規約

## 基盤の API の仕様

リクエスト・レスポンスの表は `.proto` のコメントから生成している(`make wiki-gen`)。

- [API: DeviceService(デバイス登録・トークンの発行)](device-service.md) — `voltech.v1.DeviceService` の `Register`・`IssueToken` と、認証が必要な API の呼び方
