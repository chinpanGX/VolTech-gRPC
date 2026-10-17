---
title: "基盤の版を上げる・基盤を一緒に直す"
status: current
area: guide
related: [guide/getting-started, repo/operations]
proto: []
code: [mk/codegen.mk, CHANGELOG.md]
updated: 2026-10-11
---

# 基盤の版を上げる・基盤を一緒に直す

アプリは基盤をリポジトリの URL とタグで参照している([始め方](getting-started.md))。なぜこの形にしたかは [運用(パッケージとサンプル)](../repo/operations.md)。

## 基盤の版を上げる

1. 基盤の `CHANGELOG.md` で、上げる版までの変更を読む。アプリ側で直すことがあれば、そこに書いてある
2. 次の3か所のタグを、同じ値にそろえて変える
   - `client/Packages/manifest.json` の `com.chinpangx.voltech.grpc` の URL(`#v0.4.0` の部分)
   - `server/Cargo.toml` の `voltech-grpc-*`(`tag = "v0.4.0"`)
   - `Makefile` の `VOLTECH_TAG`
3. `make proto-gen errors-gen` で C# を生成し直す(新しいタグの基盤は、このときに取ってくる)。プラグインが生成するラッパーは、同じタグの `ApiCore` に合わせて作られているので、タグを上げたら必ず生成し直す
4. サーバーをビルドし、Unity でコンパイルして、テストを流す

クライアント・サーバー・プラグインの3つは組で動く。どれか1つだけ版がずれると、生成したラッパーがコンパイルできなかったり、通信がかみ合わなかったりする。

## 基盤とアプリを一緒に直す

基盤を手元に clone し、アプリの参照を一時的に手元のパスに向ける。

- Rust: アプリの `Cargo.toml` の `[patch]` で、`voltech-grpc-*` のクレートを手元の VolTech-gRPC の `server/rust/` の下のフォルダに向ける

  ```toml
  [patch."https://github.com/chinpanGX/VolTech-gRPC"]
  voltech-grpc-auth = { path = "../../VolTech-gRPC/server/rust/auth" }
  # 使っているクレートをすべて書く
  ```

- Unity: `Packages/manifest.json` を、手元の `VolTech-gRPC/Unity/com.chinpangx.voltech.grpc` を `file:` で参照する形にする
- protoc プラグイン: アプリの Makefile の `VOLTECH_ROOT` を、手元の VolTech-gRPC に向ける(取ってくる行は動かない)

直し終わったら VolTech-gRPC にタグを打ち、アプリの参照をそのタグに戻す。
