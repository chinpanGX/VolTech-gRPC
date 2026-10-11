---
title: "概要(特徴・範囲・全体構成)"
status: confirmed
area: overview
related: [repo/operations, repo/naming, guide/getting-started, guide/roles, api/errors, api/idempotency, api/common-response, server/transaction, client/fake-testing]
proto: []
updated: 2026-10-11
---

# 概要(特徴・範囲・全体構成)

VolTech gRPC は、Unity クライアントと Rust の API サーバーを gRPC でつなぐための基盤。`.proto` を API の唯一の定義元とし、サーバー・クライアント双方の型と通信コードを生成して、型の食い違いをなくす。あわせて、API の呼び出しに共通する処理(エラー処理、冪等性、所持データの差分の反映など)を基盤として提供する。

## 特徴

- **再送しても一度だけ処理される**。通信が失敗しても、ユーザーが「再試行」を選べば同じ RequestID で送り直し、サーバーは保存済みの結果を返す([冪等性](api/idempotency.md)、[with_transaction() と冪等性](server/transaction.md))
- **1リクエスト = 1トランザクション**。業務処理、共通レスポンス項目の収集、レスポンスの保存をまとめてコミットする([with_transaction() と冪等性](server/transaction.md))
- **クライアントとサーバーを1セットで提供する**。gRPC を隠すラッパー、エラー処理、共通レスポンス項目(`player_diff`・バッジ・通知)の反映まで基盤が持つ([クライアント基盤](client/index.md))

基盤とアプリのどちらが何を持つかは [基盤とアプリの役割分担](guide/roles.md)。

## 範囲

- 対象: Unity クライアントの API 通信層と、Rust の API サーバー
- 対象外
  - **WebGL**。gRPC-Web は使わない
  - ストリーミングの RPC。v1 は Unary だけ([基本規約](api/conventions.md))
  - マスターデータの配信と、リアルタイム対戦のような常時接続の通信

## 配るもの

- **パッケージ**。リポジトリの URL とタグで配り、アプリは自分のプロジェクトから参照する([運用(パッケージとサンプル)](repo/operations.md)、名前は [リポジトリ名と命名](repo/naming.md))
  - クライアントの基盤(UPM の `com.chinpangx.voltech.grpc`)、サーバーの基盤(Rust の `voltech-grpc-*` のクレート)、ラッパーを生成する protoc プラグイン
- **サンプル**(このリポジトリの `sandbox/`。配らない)。基盤の組み込み方の実例として、アプリと同じ形で作る。簡略化したガチャで、サーバーとクライアント(Unity プロジェクト)の両方がある
  - API: 基盤のデバイス登録・認証に加えて、サインアップ・サインイン(所持データの全件取得)と、ガチャを1回引く(ジェムを消費して何か1つを付与する)
  - ガチャのレスポンスの `player_diff` にジェムの残数と入手したものを入れ、クライアントで反映する
  - 排出物は、コード内の固定テーブル(`gacha::model::ITEMS`)にする。基盤を試すのが目的なので、マスタの仕組みは持たない
  - クライアントの画面と、基盤の interface の実装例は `sandbox/UnityProject/Assets/Scripts/Sample/` にある
- サンプルをガチャにするのは、基盤の共通処理をすべて1つの機能で試せるため
  - ジェム不足が、そのまま業務エラーの例になる([エラー](api/errors.md))
  - 二重課金を防ぐという、冪等性が必要になる典型の例になる([冪等性](api/idempotency.md))
  - ジェムの減少と入手したもので差分が出る。サインインでは全件の置き換えを試せる([共通レスポンス項目](api/common-response.md))
  - 消費と付与を1つのトランザクションにまとめる必要がある([with_transaction() と冪等性](server/transaction.md))

## 全体構成

```
Unity Client (C#)
  ├─ 生成コード: メッセージ型 + gRPCクライアント (protoc + grpc_csharp_plugin)
  ├─ 生成コード: ラッパー (自作のprotocプラグイン)          ← パッケージのプラグインで生成
  ├─ 共通処理: ApiCore                                      ← パッケージ
  ├─ 認証(デバイス登録・トークン・アカウント情報の保存)    ← パッケージ
  └─ GrpcChannel + YetAnotherHttpHandler (HTTP/2)
        │
        │ gRPC (Unary)
        ▼
Rust API Server
  ├─ 状態の判定・認証の Layer                               ← パッケージ
  ├─ 共通処理: Transactions::run()                          ← パッケージ
  ├─ 生成コード: サービスtrait + メッセージ型 (tonic-build / prost)
  └─ MySQL
```
