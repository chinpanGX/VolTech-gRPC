---
title: "gRPC基盤 設計wiki"
status: index
area: root
sections: []
source: "docs/feature-grpc-foundation.md"
updated: 2026-10-10
---

# gRPC基盤 設計wiki

> 原本: `docs/feature-grpc-foundation.md`(2026-10-06 時点で章・節ごとに分割した)。以後の更新は、この wiki と `proto/` を正とする。更新のルールは [CLAUDE.md](CLAUDE.md)。

> 本書は、Unity と Rust のサーバーを gRPC でつなぐ基盤の設計ドラフト。基盤はパッケージとして提供し、テンプレートはサンプルが兼ねる。当面は1つのリポジトリで実装し、形が固まったら基盤を分ける([2.6](repo/current-plan.md)、[リポジトリ名と命名](repo/naming.md))。
>
> - **VolTech gRPC**(`voltech-grpc`): 基盤のパッケージ。Unity のクライアントの通信基盤、サーバーの認証・エラー・状態の判定・トランザクション、ラッパー生成の protoc プラグイン
> - **サンプル兼テンプレート**(`grpc-foundation-samples`): 基盤を使うアプリの雛形と、簡略化したガチャのサンプル
>
> テンプレートは簡略化したガチャをサンプルにして作り、完成後にAtlasのAPIサーバーをgRPCへ乗せ換える([1.4](overview.md))。

- ステータス: ドラフト。決まった節には【確定】を付けている。決まっていないことは[9章](open-questions.md)、採らなかった案は[8章](decisions/index.md)、参考資料は[10章](references.md)にまとめている
- 実装の進め方: パッケージ・リポジトリの分け方は後で検討する。当面は1つのリポジトリで、[2.6](repo/current-plan.md) の方針で実装を進める
- 文中の「学マス資料」「シャドバWB資料」は[10章](references.md)の資料を指す

## 読む人ごとの入口

- **基盤を使ってアプリを作る人**: [利用者向けガイド](guide/index.md) — 始め方、API の足し方、クライアント・サーバーのリファレンス、ツール
- **基盤そのものを直す人**: [基盤開発者向けガイド](dev/index.md) — コードの地図、直すときの流れ、protoc プラグインの中身

## ページ一覧

状態: `confirmed` 確定 / `deferred` 保留(別リポジトリ案) / `current` 当面の方針 / `appendix` 付録(基盤の設計とは別)

### [利用者向けガイド](guide/index.md)

| ページ | 状態 | 内容 |
|---|---|---|
| [始め方](guide/getting-started.md) | `current` | テンプレートを clone して、サーバーとクライアントを動かすまで |
| [API を足す](guide/add-api.md) | `current` | `.proto` → 生成 → サーバーのハンドラー → クライアントの呼び出し |
| [クライアントのリファレンス](guide/client-reference.md) | `current` | `ApiCore`・ラッパー・アプリが実装する interface・テスト用の部品 |
| [サーバーのリファレンス](guide/server-reference.md) | `current` | `voltech-grpc-*` のクレートと組み込み方 |
| [ツール](guide/tools.md) | `current` | protoc プラグイン・通信の Viewer・make のコマンド |

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
| [運用(パッケージとテンプレート)](repo/operations.md) | `deferred` | パッケージ(バージョン参照)とテンプレート(clone)の2形態での提供と、その運用 |
| [リポジトリ名と命名](repo/naming.md) | `current` | リポジトリ・パッケージ・クレート・.proto パッケージの命名と理由 |
| [サーバーの分け方](repo/server-split.md) | `deferred` | モジュラーモノリスの採用と、プロセスを分ける基準 |
| [フォルダ構成](repo/folder-layout.md) | `deferred` | テンプレート/パッケージのフォルダ構成と、クレート依存の向き |
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
| [通信の Viewer(エディタ拡張)](client/request-viewer.md) | `current` | リクエストとレスポンスのヘッダー・Body(JSON)を1件ずつ見るウィンドウ |
| [サンプルのクライアント(画面)](client/sample.md) | `current` | タイトル・ガチャの画面と、エラーダイアログ・画面遷移の実装例 |

### [開発運用](ops/index.md)

| ページ | 状態 | 内容 |
|---|---|---|
| [環境構築とよく使うコマンド](ops/setup.md) | `current` | 必須のツール、.proto の生成、make のコマンド、接続の確かめ方 |
| [.proto の管理](ops/proto-management.md) | `confirmed` | buf による lint / breaking / generate |
| [デバッグ](ops/debugging.md) | `confirmed` | Postman・grpcurl・gRPC リフレクション |
| [テスト](ops/testing.md) | `confirmed` | 結合テストの観点と、実行するタイミング |
| [パッケージとテンプレートを同時に直すとき](ops/local-override.md) | `confirmed` | リポジトリを分けた場合の手元参照の手順 |

### 付録

| ページ | 状態 | 内容 |
|---|---|---|
| [付録: Atlasの乗せ換え](appendix/atlas-migration.md) | `appendix` | Atlas の REST から gRPC への乗せ換えメモ |

### 判断の記録

- [未確定・保留](open-questions.md) — 決まっていないこと
- [採らなかった案(索引)](decisions/index.md) — 検討して見送った案と、その理由
- [参考資料](references.md) — 設計の根拠にした資料とリンク
