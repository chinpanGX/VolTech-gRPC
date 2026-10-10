---
title: ".proto の管理"
status: confirmed
area: ops
sections: ["6.1"]
source: "docs/feature-grpc-foundation.md"
related: []
proto: []
updated: 2026-10-06
---

# .proto の管理

- **buf** を使う。`buf lint`(書き方)、`buf format`(整形)、`buf breaking`(後方互換を壊す変更の検出)
- C#の生成は `buf generate` で行う(protoc組み込みのC#生成、grpc_csharp_plugin、ラッパー生成のプラグイン)。Rustは `build.rs` で生成する(protox で `.proto` を読み込み、tonic-prost-build で生成する。protoc は要らない)
- 一度使ったフィールド番号・名前は再利用しない(消したら `reserved` にする)
