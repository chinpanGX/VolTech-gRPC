---
title: "クライアント基盤(C# / Unity)"
status: index
area: client
sections: ["5"]
source: "docs/feature-grpc-foundation.md"
updated: 2026-10-07
---

# クライアント基盤(C# / Unity)

> 原本の見出し: 5. クライアント基盤 【確定】

基盤は、**gRPCを呼び出し側から隠すラッパー**と、**共通処理をまとめた `ApiCore`** を提供する(DeNAの内製基盤Takashoと同じ考え方)。型名(`ApiCore`、`ApiResult` など)は、実装するときに調整してよい。

## ページ

- [構成](structure.md) — 依存パッケージ、Core / Auth / Unity 層の分割、本体の決まり
- [ラッパー](wrapper.md) — 生成ラッパーの形と、通信をキャンセルできない方針
- [ApiCore の共通処理](api-core.md) — ApiCore が担う共通処理の一覧
- [エラー処理](error-handling.md) — エラー変換、Result 型、自動リトライをしない方針、エラーハンドラー
- [認証とアカウント情報](auth-account.md) — 自動のデバイス登録・トークン発行と、アカウント情報の暗号化保存
- [共通レスポンス項目の反映](response-items.md) — Applier の役割と、反映処理で例外が起きたときの後始末
- [接続設定](connection.md) — deadline・タイムアウト・受信サイズ・接続先
- [サーバーが無いときの差し替えとテスト](fake-testing.md) — FakeCallInvoker と汎用テスト
- [通信の Viewer(エディタ拡張)](request-viewer.md) — リクエストとレスポンスのヘッダー・Body(JSON)を1件ずつ見るウィンドウ
- [サンプルのクライアント(画面)](sample.md) — タイトル・ガチャの画面と、エラーダイアログ・画面遷移の実装例
