---
title: "パッケージとテンプレートを同時に直すとき"
status: confirmed
area: ops
related: [repo/current-plan]
proto: []
updated: 2026-10-10
---

# パッケージとテンプレートを同時に直すとき

> テンプレートから作ったアプリは、基盤を手元のパスで参照しているので、この手順は要らない。基盤をタグで参照している既存のアプリ(Atlas など)で、基盤とアプリを同時に直すときの手順([運用](../repo/operations.md))。

- Rust: テンプレートの `Cargo.toml` の `[patch]` で、`voltech-grpc-*` のクレートを手元の `voltech-grpc` のフォルダに向ける
- Unity: `Packages/manifest.json` を、一時的に手元の `voltech-grpc/unity` を `file:` で参照する形にする
- 直し終わったら VolTech-gRPC にタグを打ち、アプリの参照をそのタグに戻す
