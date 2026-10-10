---
title: "サーバー基盤(Rust)"
status: index
area: server
sections: ["4"]
source: "docs/feature-grpc-foundation.md"
updated: 2026-10-06
---

# サーバー基盤(Rust)

> 原本の見出し: 4. サーバー基盤 【確定(クレートの細かい分け方を除く)】

## ページ

- [レイヤー構成](layers.md) — api → service → model の3層と、トランザクションを型で強制する理由
- [リクエストの処理の流れ](request-flow.md) — Layer の順序(アップデート→メンテ→認証→日付変更)とタイムアウト
- [with_transaction() と冪等性](transaction.md) — トランザクションの手順と、最新1件保存による冪等性の実装
- [コレクター(差分・バッジ・通知)](collectors.md) — 共通レスポンス項目を集めて詰める仕組み
- [認証(サーバー)](auth.md) — デバイス登録・トークン・AuthLayer・PlayerResolver
- [サーバーの状態(メンテナンス・強制アップデート・日付変更)](status.md) — 状態の設定ファイル、判定、後から足すもの
