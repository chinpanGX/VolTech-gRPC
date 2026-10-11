---
title: "付録: 基盤開発者向け"
status: index
area: appendix
related: [guide/index, repo/current-plan]
proto: []
updated: 2026-10-10
---

# 付録: 基盤開発者向け

VolTech gRPC(基盤)そのもの(このリポジトリ)を直す人向けのページ。アプリを作る人は読まなくてよい([アプリ開発者向けガイド](../guide/index.md))。

## ページ

- [コードの地図](code-map.md) — どこに何があるか、依存の向き、テストの場所
- [基盤を直すときの流れ](workflow.md) — 確かめること、テストの流し方、wiki の更新、コミット
- [protoc プラグインの中身](protoc-plugin.md) — `protoc-gen-voltech-grpc` のファイルと決まりごと
- [環境構築とよく使うコマンド](setup.md) — このリポジトリで要るツール、`make` のコマンド、sandbox での確かめ方
- [テスト](testing.md) — 基盤のテストの観点と、流すタイミング

設計(なぜそうしたか)は、[概要](../overview.md) から辿れる各ページにある。決まっていないことは [未確定・保留](../open-questions.md)。
