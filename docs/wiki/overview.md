---
title: "概要(目的・範囲・成果物・進め方・全体構成)"
status: confirmed
area: overview
sections: ["1.1", "1.2", "1.3", "1.4", "1.5"]
related: [repo/operations, repo/naming, repo/folder-layout, repo/current-plan, api/errors, api/idempotency, api/common-response, server/transaction, client/fake-testing, appendix/atlas-migration]
proto: []
updated: 2026-10-10
---

# 概要(目的・範囲・成果物・進め方・全体構成)

### 1.1 目的

Unityクライアントと Rust APIサーバー間の通信を、REST(OpenAPI)から gRPC に置き換えるための基盤を作る。
`.proto` をAPIの唯一の定義元とし、サーバー・クライアント双方の型と通信コードを生成して、型の食い違いをなくす。
あわせて、API呼び出しに共通する処理(エラー処理、冪等性、ユーザーデータ差分の反映など)を基盤として提供する。

基盤の主な特徴:

- **再送しても一度だけ処理される**。通信が失敗しても、ユーザーが「再試行」を選べば同じRequestIDで送り直し、サーバーは保存済みの結果を返す([3.6](api/idempotency.md)、[4.3](server/transaction.md))
- **1リクエスト = 1トランザクション**。業務処理、共通レスポンス項目の収集、レスポンスの保存をまとめてコミットする([4.3](server/transaction.md))
- **クライアントとサーバーを1セットで提供する**。gRPCを隠すラッパー、エラー処理、共通レスポンス項目(playerDiff・バッジ・通知)の反映まで基盤が持つ([5章](client/index.md))

### 1.2 範囲

- 対象
  - **APIサーバー(Rust)の通信方式を REST から gRPC に置き換える**。サーバーの言語は Rust のまま
  - UnityクライアントのAPI通信層
- 対象外
  - マスターデータの生成パイプライン(master-data-pipeline)。**変更しない**
  - リアルタイム対戦サーバー(C# / MagicOnion)
  - **WebGL**。gRPC-Web は使わない

### 1.3 成果物

> リポジトリと名前は、リポジトリを分けた場合の形([2.1](repo/operations.md)〜[2.4](repo/folder-layout.md))。当面は1つのリポジトリで、同じ中身を実装する([2.6](repo/current-plan.md))。

- **パッケージのリポジトリ**(`voltech-grpc`。[2.2](repo/naming.md))。バージョン(タグ)を付けて配る([2.1](repo/operations.md))
  - クライアントの基盤(UPM の `com.chinpangx.voltech.grpc`)、サーバーの基盤(Rust の `voltech-grpc-*` のクレート)、ラッパー生成の protoc プラグイン
- **テンプレート**は、下のサンプルが兼ねる(`grpc-foundation-samples`。[2.2](repo/naming.md))。アプリはこれを clone して作る([2.1](repo/operations.md))
  - アプリの雛形(Unity プロジェクト・サーバー・`.proto`)を含む、そのまま開発を始められる雛形
- どちらもAtlasとは独立して管理する
- **サンプルのリポジトリ**。テンプレートを使う最初のアプリとして、普通のアプリと同じ手順([2.1](repo/operations.md))で作る
  - 中身は **簡略化したガチャ**で、**サーバーとクライアント(Unityプロジェクト)の両方**を用意する
  - API: 基盤のデバイス登録・認証に加えて、サインイン(所持データの全件取得)と、ガチャを1回引く(ジェムを消費して何か1つを付与する)
  - ガチャのレスポンスの `player_diff` にジェムの残数と入手したものを入れ、クライアントで反映する
  - ガチャの排出物は、コード内の固定テーブル(`gacha::model::ITEMS`)にする。基盤を試すのが目的なので、マスタの仕組みは持たない
  - サインアップを入れる。サインアップは何も受け取らず、サインインはプレイヤーとジェムを返す。未作成なら `PLAYER_NOT_FOUND`(`app/v1/player.proto`)
  - クライアントに入れるもの(画面の実装例は grpc-foundation-samples にある):
    - タイトル画面: デバイス登録とトークンの発行(`ApiCore` が自動で行う)の後、サインインで全件を取得する(実装済み)
    - ガチャ画面(ホームを兼ねる): playerDiff でジェムの残数と所持品が更新されるところと、ジェム不足(業務エラー)のダイアログを見せる(実装済み)
    - 基盤のインターフェースの実装例: `IErrorDialogView`・`IErrorNavigator`(実装済み)、`IPlayerDiffApplier`・`IBadgeApplier`・`INotificationApplier`(実装済み)、カスタムエラーハンドラー
    - テスト: 偽のサーバー(`FakeGachaServer` など)と、基盤の汎用のテスト([5.8](client/fake-testing.md))
- REST時代の成果物の残し方
  - `api-codegen`(OpenAPI → Unity向けDTO・通信クライアントの生成ツール)は、独立したリポジトリとして残す
  - AtlasのREST版(サーバーとクライアント)は、乗せ換えを始める前の状態をブランチとして残す

### 1.4 進め方

1. **パッケージとテンプレートを作る**: 続けて、テンプレートからサンプルのリポジトリを作り、ガチャを書く
2. **AtlasのREST版を残す**: 乗せ換えを始める前のコミットから、REST版を残すブランチを作る
3. **Atlasを乗せ換える**: テンプレートの基盤をAtlasに取り込み、APIを順にgRPCへ移す([7章](appendix/atlas-migration.md))

- サンプルをガチャにするのは、基盤の共通処理をすべて1つの機能で試せるため
  - ジェム不足が、そのまま業務エラーの例になる([3.5](api/errors.md))
  - 二重課金を防ぐという、冪等性が必要になる典型の例になる([3.6](api/idempotency.md))
  - ジェムの減少と入手したもので差分が出る。サインインでは全件の置き換えを試せる([3.9](api/common-response.md))
  - 消費と付与を1つのトランザクションにまとめる必要がある([4.3](server/transaction.md))

### 1.5 全体構成

```
Unity Client (C#)
  ├─ 生成コード: メッセージ型 + gRPCクライアント (protoc + grpc_csharp_plugin)
  ├─ 生成コード: ラッパー (自作のprotocプラグイン)          ← パッケージのプラグインで生成(3.4、5.2)
  ├─ 共通処理: ApiCore                                      ← パッケージ(5.3)
  ├─ 認証(デバイス登録・トークン・アカウント情報の保存)    ← パッケージ(5.5)
  └─ GrpcChannel + YetAnotherHttpHandler (HTTP/2)
        │
        │ gRPC (Unary)
        ▼
Rust API Server
  ├─ 状態の判定・認証の Layer                               ← パッケージ(4.5、4.6)
  ├─ 共通処理: with_transaction()                           ← テンプレートの基盤(4.3)
  ├─ 生成コード: サービスtrait + メッセージ型 (tonic-build / prost)
  └─ MySQL
```

## 採らなかった案

| 項目        | 採らなかった案                         | 理由                                         |
| --------- | ------------------------------- | ------------------------------------------ |
| サンプルの置き場所 | テンプレートのアプリの雛形に置き、アプリを作るときに書き換える | サンプルが手元に残らず、基盤側がサンプルを直すと取り込みで衝突する          |
| サンプルの置き場所 | テンプレートの中の `samples/` に置く        | 基盤を取り込んでアプリを作る手順を試せない                      |
| サンプルの機能   | Atlasのチャット機能                    | 業務エラーがほとんど無く、所持データを変えないので playerDiff を試せない |
