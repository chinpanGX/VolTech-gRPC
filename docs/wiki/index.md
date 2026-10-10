---
title: "VolTech gRPC wiki"
status: index
area: root
updated: 2026-10-11
---

# VolTech gRPC wiki

Unity のクライアントと Rust のサーバーを gRPC でつなぐための基盤。`.proto` に API を書くと、サーバーとクライアントの型と呼び出しのコードが生成され、通信のたびに要る処理は基盤が受け持つ。

| 基盤がすること | アプリが書くこと |
|---|---|
| デバイス登録・トークンの発行・認証(初回の通信で自動) | API の `.proto` |
| エラーの変換と、ダイアログ・画面遷移の振り分け | ハンドラー(業務処理)と DB のテーブル |
| 再送しても一度だけ処理する(RequestID) | ダイアログの見た目と画面遷移 |
| 1リクエスト = 1トランザクション | 所持データの差分(`player_diff`)の反映処理 |
| メンテナンス・強制アップデート・日付変更の判定 | |
| 通信の記録と、エディタでの確認(gRPC Monitor) | |

基盤はパッケージとして配り、アプリは自分のプロジェクトからタグで参照する。組み込み方の実例は、このリポジトリの `sandbox/`(基盤の機能を1つずつ試す RPC と、ガチャのサンプル)にある。

## はじめて使う人はこの順に読む

1. [始め方](guide/getting-started.md) — 基盤をパッケージとして参照し、sandbox のサンプルを動かす
2. [API を足す](guide/add-api.md) — `.proto` を書き、生成し、サーバーとクライアントから使う
3. 組み込むときに引く: [クライアントのリファレンス](guide/client-reference.md)・[サーバーのリファレンス](guide/server-reference.md)・[ツール](guide/tools.md)

## やりたいことから探す

| やりたいこと | 読むページ |
|---|---|
| 業務エラー(ジェム不足など)を返したい | [エラー](api/errors.md) |
| エラーを画面で自分で処理したい / ダイアログの見た目を変えたい | [エラー処理](client/error-handling.md) |
| 課金のような RPC を二重に実行させたくない / 読むだけの RPC にしたい | [冪等性](api/idempotency.md) |
| 所持データの変化をクライアントに反映したい | [共通レスポンス項目](api/common-response.md)・[反映](client/response-items.md) |
| メンテナンス・強制アップデートにしたい | [サーバーの状態](server/status.md) |
| 接続先・タイムアウトを変えたい | [接続設定](client/connection.md) |
| サーバー無しでクライアントを試したい・テストしたい | [サーバーが無いときの差し替えとテスト](client/fake-testing.md) |
| 通信の中身を見たい | [ツール](guide/tools.md)(gRPC Monitor)・[デバッグ](ops/debugging.md)(grpcurl) |
| 環境を作りたい・make のコマンドを知りたい | [環境構築とよく使うコマンド](ops/setup.md) |
| upstream から基盤を取り込みたい | [運用](repo/operations.md) |

## 基盤そのものを直す人

[基盤開発者向けガイド](dev/index.md) から読む(コードの地図、直すときの流れ)。wiki の書き方は [CLAUDE.md](CLAUDE.md)。決まっていないことは [未確定・保留](open-questions.md)。

## ページ一覧

下の「設計」の各章は、なぜその形にしたかを書いたページ。使い方だけを知りたいときは、上の入口から読めばよい。

状態: `confirmed` 確定 / `current` 当面の方針(あとで変わり得る) / `deferred` 保留 / `appendix` 付録

### [利用者向けガイド](guide/index.md)

| ページ | 状態 | 内容 |
|---|---|---|
| [始め方](guide/getting-started.md) | `current` | 基盤をパッケージとして参照し、sandbox のサンプルを動かすまで |
| [API を足す](guide/add-api.md) | `current` | `.proto` → 生成 → サーバーのハンドラー → クライアントの呼び出し |
| [クライアントのリファレンス](guide/client-reference.md) | `current` | `ApiCore`・ラッパー・アプリが実装する interface・テスト用の部品 |
| [サーバーのリファレンス](guide/server-reference.md) | `current` | `voltech-grpc-*` のクレートと組み込み方 |
| [ツール](guide/tools.md) | `current` | protoc プラグイン・gRPC Monitor・make のコマンド |

### [基盤開発者向けガイド](dev/index.md)

| ページ | 状態 | 内容 |
|---|---|---|
| [コードの地図](dev/code-map.md) | `current` | どこに何があるか、依存の向き、テストの場所 |
| [基盤を直すときの流れ](dev/workflow.md) | `current` | 確かめること、テストの流し方、wiki の更新、コミット |
| [protoc プラグインの中身](dev/protoc-plugin.md) | `current` | `protoc-gen-voltech-grpc` のファイルと決まりごと |

### 概要

| ページ | 状態 | 内容 |
|---|---|---|
| [概要(目的・範囲・成果物・進め方・全体構成)](overview.md) | `confirmed` | REST から gRPC への置き換えの目的、範囲、成果物、進め方、全体構成図 |

### [リポジトリ構成](repo/index.md)

| ページ | 状態 | 内容 |
|---|---|---|
| [運用(パッケージとサンプル)](repo/operations.md) | `current` | 基盤をパッケージとして配り、サンプルを sandbox に置く運用 |
| [リポジトリ名と命名](repo/naming.md) | `current` | リポジトリ・パッケージ・クレート・.proto パッケージの命名と理由 |
| [サーバーの分け方](repo/server-split.md) | `deferred` | モジュラーモノリスの採用と、プロセスを分ける基準 |
| [フォルダ構成](repo/folder-layout.md) | `deferred` | 当初の案のフォルダ構成(いまの形は [コードの地図](dev/code-map.md)) |
| [基盤とアプリの境界](repo/boundary.md) | `deferred` | テーブル・.proto・エラー表示などの、基盤とアプリの責務分担 |
| [当面の実装方針](repo/current-plan.md) | `current` | 単一リポジトリでの framework / app 分離と、実装の中で決めたこと |

### [API定義](api/index.md)

| ページ | 状態 | 内容 |
|---|---|---|
| [基本規約(型の定義元・通信方式・nullable と oneof)](api/conventions.md) | `confirmed` | マスターと API の型の分離、v1 は Unary のみ、nullable / oneof の規約 |
| [コード生成](api/codegen.md) | `confirmed` | コード生成の流れ、自作 protoc プラグイン、カスタムオプション |
| [エラー](api/errors.md) | `confirmed` | ErrorInfo による返し方、エラーコードと action、予約コード |
| [冪等性(RequestID と idempotency_level)](api/idempotency.md) | `confirmed` | 再送しても一度だけ処理される仕組みと、RPC ごとの宣言 |
| [認証](api/auth.md) | `confirmed` | デバイス認証(ゲスト型)の流れと skip_auth |
| [メタデータ(ヘッダー)](api/metadata.md) | `confirmed` | リクエスト/レスポンスのメタデータ一覧 |
| [共通レスポンス項目](api/common-response.md) | `confirmed` | player_diff・badges・notifications の規約 |
| [API: DeviceService(デバイス登録・トークンの発行)](api/device-service.md) | `current` | 基盤の API の仕様。リクエスト・レスポンスの表は .proto から生成 |

### [サーバー基盤(Rust)](server/index.md)

| ページ | 状態 | 内容 |
|---|---|---|
| [レイヤー構成](server/layers.md) | `confirmed` | api → service → model の3層と、トランザクションを型で強制する理由 |
| [リクエストの処理の流れ](server/request-flow.md) | `confirmed` | Layer の順序(アップデート→メンテ→認証→日付変更)とタイムアウト |
| [with_transaction() と冪等性](server/transaction.md) | `confirmed` | トランザクションの手順と、最新1件保存による冪等性の実装 |
| [コレクター(差分・バッジ・通知)](server/collectors.md) | `confirmed` | 共通レスポンス項目を集めて詰める仕組み |
| [認証(サーバー)](server/auth.md) | `confirmed` | デバイス登録・トークン・AuthLayer・PlayerResolver |
| [サーバーの状態(メンテナンス・強制アップデート・日付変更)](server/status.md) | `confirmed` | 状態の設定ファイル、判定、後から足すもの |

### [クライアント基盤(C# / Unity)](client/index.md)

| ページ | 状態 | 内容 |
|---|---|---|
| [構成](client/structure.md) | `confirmed` | 依存パッケージ、Core / Auth / Unity 層の分割、本体の決まり |
| [ラッパー](client/wrapper.md) | `confirmed` | 生成ラッパーの形と、通信をキャンセルできない方針 |
| [ApiCore の共通処理](client/api-core.md) | `confirmed` | ApiCore が担う共通処理の一覧 |
| [エラー処理](client/error-handling.md) | `confirmed` | エラー変換、Result 型、自動リトライをしない方針、エラーハンドラー |
| [認証とアカウント情報](client/auth-account.md) | `confirmed` | 自動のデバイス登録・トークン発行と、アカウント情報の暗号化保存 |
| [共通レスポンス項目の反映](client/response-items.md) | `confirmed` | Applier の役割と、反映処理で例外が起きたときの後始末 |
| [接続設定](client/connection.md) | `confirmed` | deadline・タイムアウト・受信サイズ・接続先 |
| [サーバーが無いときの差し替えとテスト](client/fake-testing.md) | `confirmed` | FakeCallInvoker と汎用テスト |
| [gRPC Monitor(エディタ拡張)](client/grpc-monitor.md) | `current` | リクエストとレスポンスのヘッダー・Body(JSON)を1件ずつ見るウィンドウ |

### [開発運用](ops/index.md)

| ページ | 状態 | 内容 |
|---|---|---|
| [環境構築とよく使うコマンド](ops/setup.md) | `current` | 必須のツール、.proto の生成、make のコマンド、接続の確かめ方 |
| [.proto の管理](ops/proto-management.md) | `confirmed` | buf による lint / breaking / generate |
| [デバッグ](ops/debugging.md) | `confirmed` | Postman・grpcurl・gRPC リフレクション |
| [テスト](ops/testing.md) | `confirmed` | 結合テストの観点と、実行するタイミング |
| [パッケージとアプリを同時に直すとき](ops/local-override.md) | `confirmed` | 基盤をタグで参照しているアプリから、手元の基盤を参照する手順 |

### 付録

| ページ | 状態 | 内容 |
|---|---|---|
| [付録: Atlasの乗せ換え](appendix/atlas-migration.md) | `appendix` | Atlas の REST から gRPC への乗せ換えメモ |

### 判断の記録

- [未確定・保留](open-questions.md) — 決まっていないこと
