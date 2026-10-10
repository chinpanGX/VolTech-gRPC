---
title: "運用(パッケージとテンプレート)"
status: deferred
area: repo
related: [repo/naming, repo/current-plan, guide/getting-started, ops/local-override]
proto: [sandbox/proto/sandbox/v1/error_code.proto]
code: [sandbox/server/Cargo.toml, sandbox/UnityProject/Packages/manifest.json, buf.yaml]
updated: 2026-10-10
---

# 運用(パッケージとテンプレート)

このリポジトリ(VolTech-gRPC)を**テンプレート**にする。基盤と、アプリの雛形(`sandbox/`)を1つのリポジトリに持ち、アプリはこれを clone して作る。基盤のフォルダはパッケージの形(UPM パッケージ・機能ごとのクレート)にしてあるので、clone できない既存のアプリ(Atlas)は、タグで参照する。

| 使い方 | 対象 | 基盤の参照 | 基盤の更新 |
|---|---|---|---|
| テンプレート(基本) | 新しく作るアプリ(サンプルを含む) | 手元のパス(Cargo の `path`、UPM の `file:`、`buf.yaml` の `modules`) | upstream から merge する |
| パッケージ | 既存のアプリ(Atlas など) | タグ(Cargo の `git` + `tag`、UPM の git URL) | タグを上げる |

## テンプレート

- アプリを作るときは、このリポジトリを `git clone` し、複製元を `upstream` として登録する(手順は [始め方](../guide/getting-started.md))
- 基盤の改善は `git fetch upstream` → merge で取り込む。基盤と雛形が1つの履歴なので、組み合わせが食い違わない(基盤のタグと雛形の対応を管理しなくてよい)
- **アプリは基盤のフォルダ(`server/rust/`・`Unity/com.chinpangx.voltech.grpc/`・`tools/`)を直接直さない**。直すと upstream からの取り込みで衝突する。アプリが変えたいところは、用意した差し替え口を使う(共通エラーハンドラーの差し替え、`IErrorDialogView`・`IErrorNavigator`、反映処理、カスタムエラーハンドラー、`IAccountStore`、接続設定、サーバーの `PlayerResolver`。[クライアント基盤](../client/index.md))。足りない場面が出たら、upstream の基盤に差し替え口を足す
- アプリのコードは雛形(`sandbox/`)の中に書く。基盤の側は雛形に依存しないので、アプリの変更と基盤の取り込みは別のフォルダに分かれる
- サンプル(grpc-foundation-samples)もこの手順で作り、基盤の改善を取り込む。基盤を取り込んでアプリを作る手順を、Atlasより先にサンプルで試すため
  - サンプルの業務エラーコードは、サンプルのリポジトリの `error_code.proto` にだけ入る(テンプレートの `error_code.proto` は予約コードだけ)
- テストは、サンプルに頼らずこのリポジトリで行う。クライアントの基盤のテスト(Unity の EditMode テスト)、サーバーの基盤のテスト、雛形を使った結合テスト(デバイス登録・トークン・冪等性・エラー・メンテナンスの判定など。[開発運用](../ops/index.md))

## パッケージ

- Rust は Git の依存(`tag` でバージョンを固定)、Unity は UPM の Git URL(`?path=` とタグ)で参照する。protoc プラグインは dotnet tool として入れる
- クライアントとサーバーの認証、プラグインと `ApiCore` は組で動くので、1つのリポジトリから同じタグで出す(`device.proto` や、ラッパーと `ApiCore` の食い違いが起きない)
- 基盤を同時に直すときは、手元のパスに差し替える([パッケージとテンプレートを同時に直すとき](../ops/local-override.md))
- 基盤はアプリのコードに依存しない。サーバーでアプリとつなぐのは `PlayerResolver` の trait だけ
- タグの付け方とリリースの手順は未確定([未確定・保留](../open-questions.md) #16)

## 実装で決めたこと

- テンプレートは、基盤を分けた後の VolTech-gRPC にする(2026-10-10)。当初はサンプルにテンプレートを兼ねさせ、基盤はタグで参照する形にしていたが、アプリを作るたびにサンプルのコードを消すことになり、基盤のタグと雛形の組み合わせを合わせる手間もかかるため改めた
- 雛形は、基盤の動作確認に使っていた `sandbox/` にする。すでに基盤を手元のパスで参照し、デバイス登録からトランザクションまでを通す最小のサーバーと Unity プロジェクトになっている。名前と、雛形に残す確認用の RPC の扱いは未確定([未確定・保留](../open-questions.md) #23)
