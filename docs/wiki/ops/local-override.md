---
title: "パッケージとテンプレートを同時に直すとき"
status: confirmed
area: ops
sections: ["6.4"]
related: [repo/current-plan]
proto: []
updated: 2026-10-10
---

# パッケージとテンプレートを同時に直すとき

> リポジトリを分けた場合の手順。当面は1つのリポジトリ・1つの Cargo ワークスペースなので、この手順は要らない([2.6](../repo/current-plan.md))。

- Rust: テンプレートの `Cargo.toml` の `[patch]` で、`voltech-grpc-*` のクレートを手元の `voltech-grpc` のフォルダに向ける
- Unity: `Packages/manifest.json` を、一時的に手元の `voltech-grpc/unity` を `file:` で参照する形にする
- 直し終わったら `voltech-grpc` にタグを打ち、テンプレートの参照をそのタグに戻す
