---
title: "リポジトリ構成"
status: index
area: repo
updated: 2026-10-11
---

# リポジトリ構成

> 基盤は VolTech-gRPC に分け、パッケージとして配る。サンプルはこのリポジトリの sandbox に置く([運用](operations.md))。[フォルダ構成](folder-layout.md) と [当面の実装方針](current-plan.md) には、分ける前の案と経緯が残っている。

## ページ

- [運用(パッケージとサンプル)](operations.md) — 基盤をパッケージとして配り、サンプルを sandbox に置く運用
- [リポジトリ名と命名](naming.md) — リポジトリ・パッケージ・クレート・.proto パッケージの命名と理由
- [サーバーの分け方](server-split.md) — モジュラーモノリスの採用と、プロセスを分ける基準
- [フォルダ構成](folder-layout.md) — 当初の案のフォルダ構成と、クレート依存の向き
- [基盤とアプリの境界](boundary.md) — テーブル・.proto・エラー表示などの、基盤とアプリの責務分担
- [当面の実装方針](current-plan.md) — 単一リポジトリでの framework / app 分離と、実装の中で決めたこと
