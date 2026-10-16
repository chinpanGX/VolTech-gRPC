---
title: "ゲームログ"
status: current
area: server
related: [server/transaction, server/logging, guide/server-reference, guide/add-api]
proto: []
code: [server/rust/transaction/src/game_log.rs, server/rust/transaction/src/context.rs, server/rust/transaction/migrations, sandbox/server/player/src/service, sandbox/server/gacha/src/service, sandbox/server/server/src/config.rs]
updated: 2026-10-16
---

# ゲームログ

ゲームログは、所持データの変化(ガチャでのジェムの消費、アイテムの付与など)の履歴。問い合わせ(「ジェムが減った」)・補填・分析に使う。サーバーの動きを追う [運用ログ](logging.md) とは別の種類として扱う。

## 書き方

`Transactions::run` に渡す処理が受け取る `TxContext` の `write_game_log` で書く。アプリは、所持データを変えたところで、何がどれだけ変わったかを渡す。

| 値 | 渡す人 | 内容 |
|---|---|---|
| 種別 | アプリ | 例: `gems`・`item` |
| 対象の ID | アプリ(任意) | アイテムの ID など。名前は変わることがあるので、ID を書く |
| 増減・更新後の値 | アプリ | 例: `-300` と `2700` |
| 理由 | アプリ | 例: `gacha_draw`・`sign_up` |
| `player_id` | 基盤 | `AuthLayer` が入れた `AuthContext` から取る |
| `request_id`・RPC・時刻 | 基盤 | `x-request-id`(形が正しいときだけ)、`rpc.method` と同じ名前、書いた時刻 |

- **ゲームログは `TxContext` からだけ書ける**。書いたゲームログは `run` の中にため、処理が成功してコミットするときだけ出力先へ書く。ロールバックしたとき、再送で保存済みのレスポンスを返したとき(処理を呼ばない)に残らないことを、型で保証するため
- `player_id` は、`run` の引数ではなく `AuthContext` から取る。サインアップ(`player_id` を渡さない `run`)の初期データの付与も残すため
- `NO_SIDE_EFFECTS` の RPC で書くと、INTERNAL にする。読み取り専用のトランザクションで所持データは変わらないので、書くのは宣言か実装の間違いのため
- `AuthLayer` を付けていないサービスで書いても INTERNAL にする(誰のログか分からないため)

## 出力先

出力先は `GameLogSink` で選び、`Transactions::new` に渡す。`LogConfig` に入れないのは、書くのが `voltech-grpc-transaction` で、ログのクレートからトランザクションのクレートへ依存の向きを逆にしないため。

| 出力先 | 書くとき | 向いている構成 |
|---|---|---|
| `GameLogSink::Database` | 業務の処理と同じトランザクションで、`game_logs` テーブルに書く。ロールバックしたら残らない | DB で問い合わせ・補填を調べる |
| `GameLogSink::Stdout` | コミットに成功してから、標準出力に JSON で出す | ログの収集・分析のサービスへ送る(例: CloudWatch Logs → S3 → Athena、Cloud Logging → BigQuery) |

### `game_logs` テーブル

基盤のマイグレーション(`voltech_grpc_transaction::migrate`)が作る。種別ごとにテーブルを分けず、種別で形の変わりうる値は `body` の JSON に入れる。種別を足すたびにテーブルを足さなくてよいようにするため。

| 列 | 内容 |
|---|---|
| `log_id` | ログの ID(連番) |
| `player_id` | プレイヤー(`BINARY(16)`) |
| `request_id` | `x-request-id`。無い・形が違うときは `NULL` |
| `method` | RPC の名前(例: `sandbox.v1.GachaService/Draw`) |
| `kind` | 種別 |
| `body` | `target_id`・`delta`・`after`・`reason` の JSON |
| `created_at` | 書いた時刻 |

- 問い合わせで、プレイヤーの履歴を時刻の順に引くので、`(player_id, created_at)` にインデックスを付ける

### 標準出力

target `voltech_grpc_log::game`、メッセージ `game log` で、1件を1行で出す。項目は `log_type`(`game`)・`player_id`・`request_id`・`rpc.method`・`kind`・`target_id`・`delta`・`after`・`reason`。

- **`RUST_LOG` のフィルターで消えない**。運用ログのレベルの設定で、問い合わせ・補填に使う履歴が消えないようにするため
- 運用ログと同じ流れで送られるので、`log_type` で見分けて分けて保存する
- **コミットに成功してから出すので、直後にプロセスが落ちると残らない**(書いた業務データはコミット済み)。残らないのは多くても1回分で、DB の業務データとは食い違いうる。食い違いを許せないなら `Database` を選ぶ

## 実装で決めたこと

- `TxContext` は、トランザクションを `ctx.tx()`(`&mut sqlx::Transaction`)で渡す。今までの `tx` を `ctx.tx()` に置き換えれば、同じように使える
- `write_game_log` は `Result<(), Internal>` を返す。業務の本体の関数(`Result<_, Internal>`)でも、ハンドラー(`Result<_, Status>`)でも `?` で書ける
- sandbox では、所持データを変える関数(`player::service::create`・`consume_gems`・`add_item`・`increment_counter`)の中で書く。サインアップは、プレイヤーを作ったとき(作成済みでないとき)だけ書く
