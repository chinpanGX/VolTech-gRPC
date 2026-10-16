---
title: "サーバー基盤(Rust)"
status: index
area: server
updated: 2026-10-17
---

# サーバー基盤(Rust)

## ページ

- [レイヤー構成](layers.md) — api → service → model の3層と、トランザクションを型で強制する理由
- [リクエストの処理の流れ](request-flow.md) — Layer の順序(アップデート→メンテ→認証→日付変更)とタイムアウト
- [運用ログ](logging.md) — RPC ごとの span とアクセスログ、JSON の項目、設定
- [with_transaction() と冪等性](transaction.md) — トランザクションの手順と、最新1件保存による冪等性の実装
- [ゲームログ](game-log.md) — 所持データの変化の履歴。数量の変化とそれ以外の形、run の中と外からの書き方、出力先(DB・標準出力)
- [コレクター(差分・バッジ・通知)](collectors.md) — 共通レスポンス項目を集めて詰める仕組み
- [認証(サーバー)](auth.md) — デバイス登録・トークン・AuthLayer・PlayerResolver
- [サーバーの状態(メンテナンス・強制アップデート・日付変更)](status.md) — 状態の設定ファイル、判定、後から足すもの
