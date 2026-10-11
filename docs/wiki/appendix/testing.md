---
title: "テスト"
status: confirmed
area: appendix
related: [client/error-handling]
proto: []
updated: 2026-10-11
---

# テスト

基盤(このリポジトリ)のテスト。

- サーバーの結合テストは、tonic が生成するクライアントで書く(実際のクライアントと同じ経路を通る)。基盤の動作として次を確かめる
  - 同じRequestIDで2回送ると、2回目は保存済みのレスポンスが返り、業務データは1回分しか変わらない
  - 同じRequestIDで別のメソッドを呼ぶと、エラーになる
  - `NO_SIDE_EFFECTS`・`IDEMPOTENT` のRPCは、レスポンスが保存されない
  - `skip_auth` のサービスはトークンなしで呼べ、それ以外は `UNAUTHENTICATED` になる
- このリポジトリでは、クライアントの基盤のテスト(Unity の EditMode テスト)、サーバーの基盤のテスト、sandbox を使った結合テスト(ガチャのサンプルを含む)を回す。アプリのテストの観点は [API を足す](../guide/add-api.md)

| タイミング | 実行するもの |
|---|---|
| コードを書いている最中 | Roslynアナライザー(カスタムエラーハンドラーの担当コード。[エラー処理](../client/error-handling.md)) |
| `.proto` を変えたとき | `buf lint` / `buf breaking`(`buf generate` の前) |
| プッシュする前(手元) | `make test`(基盤と sandbox の `cargo test`。docker-compose の MySQL を使う)と、Unity の EditMode テスト(クライアントの本体と Unity 用の層。[基盤を直すときの流れ](workflow.md)) |
| プルリクエスト(CI) | サーバーのテストと buf を GitHub Actions で実行する(MySQLもコンテナで立てる)。Unity のテストは、CIでのライセンス認証の設定に手間がかかるので、v1では手元だけ(クライアントの本体も含む) |
