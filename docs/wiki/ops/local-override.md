---
title: "パッケージとアプリを同時に直すとき"
status: confirmed
area: ops
related: [repo/current-plan]
proto: []
updated: 2026-10-11
---

# パッケージとアプリを同時に直すとき

> 基盤をタグで参照しているアプリで、基盤とアプリを同時に直すときの手順([運用](../repo/operations.md))。このリポジトリの sandbox は、はじめから手元のパスで参照しているので要らない。

- Rust: アプリの `Cargo.toml` の `[patch]` で、`voltech-grpc-*` のクレートを手元の `voltech-grpc` のフォルダに向ける
- Unity: `Packages/manifest.json` を、一時的に手元の `voltech-grpc/unity` を `file:` で参照する形にする
- 直し終わったら VolTech-gRPC にタグを打ち、アプリの参照をそのタグに戻す
