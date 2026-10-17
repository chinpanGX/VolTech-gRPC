---
title: "運用(パッケージとサンプル)"
status: current
area: repo
related: [repo/naming, guide/getting-started, guide/upgrade]
proto: []
code: [tools/protoc-gen-voltech-grpc/src, sandbox/server/Cargo.toml, sandbox/UnityProject/Packages/manifest.json, Unity/com.chinpangx.voltech.grpc/package.json, server/rust/Cargo.toml, CHANGELOG.md, .github/workflows/release.yml]
updated: 2026-10-17
---

# 運用(パッケージとサンプル)

基盤は**パッケージ**として配る。配る場所は、公開(public)の GitHub のリポジトリだけにする(crates.io・NuGet・OpenUPM には出さない)。アプリは自分のプロジェクトを作り、基盤をリポジトリの URL とタグで参照する。このリポジトリ(VolTech-gRPC)は、基盤と、組み込み方の実例(sandbox)を持つ。アプリがこのリポジトリを clone して、その中で作ることはしない。

| もの | 置き場所 | 使い方 |
|---|---|---|
| 基盤 | `server/rust/`(クレート)、`Unity/com.chinpangx.voltech.grpc/`(UPM パッケージ)、`tools/protoc-gen-voltech-grpc/`(protoc プラグイン) | アプリはリポジトリの URL とタグで参照する(Cargo の `git` + `tag`、UPM の git URL)。プラグインは、アプリの Makefile が同じタグの基盤を取ってきてビルドする |
| 実例(sandbox) | `sandbox/` | 基盤の機能を1つずつ試す RPC と、ガチャのサンプル。基盤を手元のパスで参照する。配らない |

## パッケージ

- Rust は Git の依存(`tag` でバージョンを固定)、Unity は UPM の Git URL(`?path=` とタグ)で参照する。protoc プラグインは、アプリの Makefile が同じタグの基盤を clone してビルドする([始め方](../guide/getting-started.md))
- アプリはパッケージのコードを直接触らない。改善は、タグのバージョンを上げて取り込む
- アプリが変えたいところは、用意した差し替え口を使う(共通エラーハンドラーの差し替え、`IErrorDialogView`・`IErrorNavigator`、反映処理、カスタムエラーハンドラー、`IAccountStore`、接続設定、サーバーの `PlayerResolver`。[クライアント基盤](../client/index.md))。足りない場面が出たら、パッケージに差し替え口を足す
- クライアントとサーバーの認証、プラグインと `ApiCore` は組で動くので、1つのリポジトリから同じタグで出す(`device.proto` や、ラッパーと `ApiCore` の食い違いが起きない)
- アプリのエラーは、アプリの `errors.toml`(アプリの domain)に書く。基盤の予約エラーは基盤が持つので、アプリに書き写さない([エラー](../api/errors.md))
- 基盤はアプリのコードに依存しない。サーバーでアプリとつなぐのは `PlayerResolver` の trait だけ
- 基盤とアプリを同時に直すときは、手元のパスに差し替える([基盤の版を上げる・基盤を一緒に直す](../guide/upgrade.md))

## リリース

基盤の版は、クレート・UPM パッケージ・プラグインで1つにそろえ、`v0.4.0` の形のタグで出す。0.x のあいだは、機能を足したときと壊れる変更のときに minor を上げ、直しだけのときに patch を上げる。

1. PR を main にマージする。変更は `CHANGELOG.md` の `## [Unreleased]` に書いておく
2. main に「chore: 0.4.0 にする」のコミットを積む。書き換えるのは次のとおり
   - `CHANGELOG.md`: `## [Unreleased]` の下に `## [0.4.0] - 日付` を足す
   - `Unity/com.chinpangx.voltech.grpc/package.json` の `version` と、`server/rust/Cargo.toml` の `workspace.package.version`
   - `docs/wiki/guide/getting-started.md` と `docs/wiki/guide/upgrade.md` に書いたタグ
   - `server/rust/Cargo.lock` と `sandbox/server/Cargo.lock`(`cargo update --workspace` で版の行だけを直す)
3. そのコミットに `v0.4.0` のタグを付け、main とタグを push する
4. タグの push で `.github/workflows/release.yml` が動き、GitHub の Release を作る(本文は GitHub が自動で作る、前のタグからの PR の一覧)

## サンプル(sandbox)

- ガチャのサンプルは sandbox に置く。基盤と同じ履歴なので、基盤の変更でサンプルが壊れたら、同じコミットのテストで気づける。依存の版の決まり(`make deps-check`)も一緒に確かめられる
- sandbox は基盤を手元のパスで参照する(Cargo の `path`、UPM の `file:`)。アプリと同じくタグで動くかは、リリースの前に確かめる
- テストはこのリポジトリで行う。クライアントの基盤のテスト(Unity の EditMode テスト)、サーバーの基盤のテスト、sandbox の結合テスト(デバイス登録・トークン・冪等性・エラー・メンテナンスの判定・ガチャなど。[テスト](../appendix/testing.md))

## 実装で決めたこと

- 基盤はリポジトリだけで配り、パッケージのレジストリ(crates.io・NuGet・OpenUPM)には出さない(2026-10-11)。Cargo も UPM も git の URL とタグで取り込めるので、リポジトリにタグを打てばリリースになる。レジストリに出すと、クレート・UPM パッケージ・プラグインを別々に公開することになり、同じタグでそろえる手間が増える

- サンプルは、別のリポジトリにせず sandbox に置く(2026-10-11)。アプリは基盤をパッケージとして参照するので、サンプルを clone の元にする必要が無い。別のリポジトリにすると、基盤を直すたびにタグを打ってサンプルの参照を上げることになり、壊れたことに気づくのも遅れる

- GitHub の Release は、手で作らずにタグの push で CI に作らせる(2026-10-17)。手で作る手順にすると、タグだけ付けて Release を作り忘れる。workflow の中身は、ほかのリポジトリと共有する [chinpanGX/workflows](https://github.com/chinpanGX/workflows) に置き、このリポジトリはそれを呼ぶだけにする
