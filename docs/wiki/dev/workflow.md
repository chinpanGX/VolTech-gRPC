---
title: "基盤を直すときの流れ"
status: current
area: dev
sections: []
related: [dev/code-map, ops/testing, ops/setup]
proto: []
code: [Makefile, .claude/skills/wiki-update/SKILL.md, scripts/wiki_affected.py]
updated: 2026-10-10
---

# 基盤を直すときの流れ

## 1. 直す前に

- 直すところの設計のページを読む(なぜその形にしたか、採らなかった案)。[コードの地図](code-map.md) から、コードと設計のページを辿れる
- 基盤(`server/rust/`・`Unity/com.chinpangx.voltech.grpc/`)は、sandbox やアプリ(grpc-foundation-samples)に依存させない。アプリの都合で基盤を直したくなったら、直接変えずに差し替え口(trait・interface)を足す([当面の実装方針](../repo/current-plan.md))
- 直した動きは sandbox で確かめる。確かめたい基盤の機能が sandbox に無ければ、`sandbox.proto` に RPC を足す(業務機能は足さない)

## 2. 直した後に確かめる

| 直したもの | 確かめること |
|---|---|
| Rust のクレート | `make fmt` → `make lint`(Clippy。警告もエラー) → `make test-rust`(MySQL を使う) |
| `.proto` | `make proto-lint` → `make proto-gen`(C# を生成し直してコミットする)。RPC やコメントを変えたら `make wiki-gen` |
| protoc プラグイン | `make plugin-test` → `make proto-gen` で生成コードの差分を確かめる([protoc プラグインの中身](protoc-plugin.md)) |
| Unity のコード | `make client-lint`(書き方) → コンパイル → EditMode テスト |

Unity のコンパイルとテストは、Unity のエディタを開いたままでも、Unity CLI で流せる(エディタに Pipeline のパッケージが入っているとき)。

```
cd sandbox/UnityProject
unity recompile                                    # コンパイルして、エラーを表示する
unity command run_tests --mode EditMode --result-only
```

- サーバーにつなぐテスト(`ApiServerTestBase` を継ぐもの)は、先に `make sandbox-server` でサーバーを起動しておく
- エディタを閉じているなら `unity test sandbox/UnityProject --mode EditMode` でも流せる
- エディタを閉じているなら、Test Runner の代わりに batchmode でも流せる(エディタで開いていると batchmode は起動できない)
- サーバーを動かしたまま Rust をビルドすると、Windows では `server.exe` がロックされて失敗する。先にサーバーを止める

まとめて流すのは `make test`(基盤と sandbox の Rust・プラグイン・wiki の確認。Unity は含まない)。

## 3. wiki を直す

コードを直したら、同じ変更の中で wiki も直す。手順はスキル `/wiki-update` にまとめてある。

1. `make wiki-affected`(コミット前)か `make wiki-affected BASE=main`(ブランチ全体)、`make wiki-affected FILES="a b"`(ファイルを直接)で、直すべきページを探す。ページの frontmatter の `code:`・`proto:` に、変わったファイルが入っているページが挙がる
2. 挙がったページを読み、ずれていれば直す。どのページにも結びつかない変更があれば、ページを足すか、既存のページの `code:` に足す
3. `make wiki-gen`(`.proto` を変えたとき) → `make wiki-lint`

ページの書き方は `docs/wiki/CLAUDE.md`。

## 4. コミット

- 1つのコミットに1つの目的。メッセージの1行目は日本語で、何をしたかを書く。本文に、何を・なぜ変えたかを箇条書きで書く
- 生成したコード(`Unity/.../Api/Generated/`)と `.meta` もコミットする
