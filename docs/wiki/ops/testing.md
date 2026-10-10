---
title: "テスト"
status: confirmed
area: ops
related: [client/error-handling]
proto: []
updated: 2026-10-11
---

# テスト

- サーバーの結合テストは、tonic が生成するクライアントで書く(実際のクライアントと同じ経路を通る)。基盤の動作として次を確かめる
  - 同じRequestIDで2回送ると、2回目は保存済みのレスポンスが返り、業務データは1回分しか変わらない
  - 同じRequestIDで別のメソッドを呼ぶと、エラーになる
  - `NO_SIDE_EFFECTS`・`IDEMPOTENT` のRPCは、レスポンスが保存されない
  - `skip_auth` のサービスはトークンなしで呼べ、それ以外は `UNAUTHENTICATED` になる
- `IDEMPOTENT` と宣言したAPIは、その結合テストで「2回呼んでも状態が同じ」を確かめる
- クライアントでは、全エラーコードの文言キーが文言データにあるかも確かめる(既定の action は `errors.toml` の生成で必ず付く)
- このリポジトリでは、クライアントの基盤のテスト(Unity の EditMode テスト)、サーバーの基盤のテスト、sandbox を使った結合テスト(ガチャのサンプルを含む)を回す。アプリは、パッケージのバージョンを上げたときに結合テストで確かめる

| タイミング | 実行するもの |
|---|---|
| コードを書いている最中 | Roslynアナライザー(カスタムエラーハンドラーの担当コード。[エラー処理](../client/error-handling.md)) |
| `.proto` を変えたとき | `buf lint` / `buf breaking`(`buf generate` の前) |
| プッシュする前(手元) | `make test` の1コマンドで、`cargo test`(結合テスト。docker-compose の MySQL を使う)と、Unity の EditMode テスト(クライアントの本体と Unity用の層) |
| プルリクエスト(CI) | サーバーのテストと buf を GitHub Actions で実行する(MySQLもコンテナで立てる)。Unity のテストは、CIでのライセンス認証の設定に手間がかかるので、v1では手元だけ(クライアントの本体も含む) |
