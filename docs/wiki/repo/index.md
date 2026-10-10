---
title: "リポジトリ構成"
status: index
area: repo
sections: ["2"]
source: "docs/feature-grpc-foundation.md"
updated: 2026-10-06
---

# リポジトリ構成

> 原本の見出し: 2. リポジトリ構成 【分け方は保留。当面は 2.6 で進める】

> [2.1](operations.md)〜[2.4](folder-layout.md)(と [1.3](../overview.md) のリポジトリ、[6.4](../ops/local-override.md) の手順)は、パッケージとテンプレートを別のリポジトリにする場合の形。パッケージ・リポジトリにどう分けるかは、実装を進めて形が固まってから検討する([9章](../open-questions.md) #17)。それまでは [2.6](current-plan.md) の方針で、1つのリポジトリで実装する。

## ページ

- [運用(パッケージとテンプレート)](operations.md) — パッケージ(バージョン参照)とテンプレート(clone)の2形態での提供と、その運用
- [リポジトリ名と命名](naming.md) — リポジトリ・パッケージ・クレート・.proto パッケージの命名と理由
- [サーバーの分け方](server-split.md) — モジュラーモノリスの採用と、プロセスを分ける基準
- [フォルダ構成](folder-layout.md) — テンプレート/パッケージのフォルダ構成と、クレート依存の向き
- [基盤とアプリの境界](boundary.md) — テーブル・.proto・エラー表示などの、基盤とアプリの責務分担
- [当面の実装方針](current-plan.md) — 単一リポジトリでの framework / app 分離と、実装の中で決めたこと
