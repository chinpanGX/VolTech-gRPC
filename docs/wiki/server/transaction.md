---
title: "with_transaction() と冪等性"
status: confirmed
area: server
related: [server/collectors, client/api-core]
proto: []
code: [server/rust/transaction]
updated: 2026-10-10
---

# with_transaction() と冪等性

`with_transaction()` は次を行う。

1. トランザクションを開始する(`NO_SIDE_EFFECTS` のRPCは読み取り専用のトランザクション)
2. 冪等性の対象のRPCなら、プレイヤーの行を `SELECT ... FOR UPDATE` でロックし、保存済みのRequestIDを読む。同じRequestIDなら、保存済みのメソッド名と一致するかを確かめ、一致すれば保存済みのレスポンスを返して終わる(一致しなければエラー)
3. ハンドラー本体(`service/` の関数)を、同じトランザクションとコレクターを渡して呼ぶ
4. 成功したら、コレクターの中身を `player_diff`・`badges`・`notifications` に詰める([コレクター](collectors.md))。対象のRPCなら、RequestID・メソッド名・レスポンスを保存(上書き)する。そしてコミットする
5. 失敗したらロールバックする。エラーのレスポンスは保存しないので、再試行すると処理し直す

| 決めたこと | 理由 |
|---|---|
| プレイヤーごとに**最新1件だけ**保存する(MySQLのテーブルに1プレイヤー1行、上書き) | 保存量がプレイヤー数で頭打ちになり、期限切れの掃除が要らない。クライアントは対象のRPCを1つずつ送る([ApiCore の共通処理](../client/api-core.md))ので、最新1件で足りる |
| 行を `SELECT ... FOR UPDATE` でロックする | 処理中に再送が届いても、最初の処理の完了を待ってから保存済みのレスポンスを返せる |
| レスポンスの保存と業務データの更新を、同じトランザクションで行う | 「業務データはコミット済み・レスポンスは未保存」の状態が生まれず、再送で二重実行されない |
| **メソッド名**も保存し、同じRequestIDで別のメソッドが呼ばれたらエラーにする | クライアントの不具合でRequestIDが使い回されたとき、型の違うレスポンスを返さない |
| `NO_SIDE_EFFECTS` は読み取り専用のトランザクションで実行する | 誤って書き込むとMySQLがその場でエラーを返すので、宣言の間違いに開発中に気づける |
| 対象かどうかは、サーバーも v1 から `.proto` の `idempotency_level` を読んで決める(`prost-types`) | クライアントとサーバーで判定がずれない。ハンドラーはすべて同じように `with_transaction()` を呼べばよい |

- ロックの対象にするため、プレイヤーの作成時に保存用の行も1行作っておく

実装で決めたこと(`server/rust/transaction`):

- `with_transaction()` は `Transactions::run(&request, player_id, handler)` として基盤のクレートに置く。テーブル `last_responses` とマイグレーションもクレートに持つ(履歴は `_transaction_migrations`)
- 冪等性の表は、起動時にアプリの FileDescriptorSet の `idempotency_level` から作る(`IdempotencyLevels`)。表に無い RPC は対象として扱う
- tonic はサーバーのハンドラーに RPC の名前を渡さないので、全サービスの外側に `RpcMethodLayer` を付け、HTTP のパスから取り出した名前を extensions に入れる
- 保存用の行は、サインアップで作るのに加えて、無ければ最初の対象のリクエストで作る(この仕組みより前に作られたプレイヤーのため)。行が無いときの `SELECT ... FOR UPDATE` は隙間のロックを取るので、それを持ったまま作ると、同時に行を作る別のプレイヤーのトランザクションとデッドロックする。いったんロールバックしてから別のトランザクションで作り、ロックを取り直す
- 冪等性の対象の RPC で `x-request-id` が無い・形が違う(英数字とハイフンの1〜64文字でない)、または同じ RequestID で別のメソッドが呼ばれたら、`INVALID_ARGUMENT` を返す(クライアントでは `UNKNOWN_ERROR` になる)
