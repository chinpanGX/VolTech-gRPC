---
title: "ゲームログ"
status: current
area: server
related: [server/transaction, server/logging, guide/server-reference, guide/add-api]
proto: []
code: [server/rust/transaction/src/game_log.rs, server/rust/transaction/src/context.rs, server/rust/transaction/migrations, server/rust/log/src/json.rs, sandbox/server/player/src/service, sandbox/server/gacha/src/service, sandbox/server/server/src/config.rs]
updated: 2026-10-17
---

# ゲームログ

ゲームログは、所持データの変化(ガチャでのジェムの消費、アイテムの付与、キャラクターの入手、編成の変更など)の履歴。問い合わせ(「ジェムが減った」)・補填・分析に使う。サーバーの動きを追う [運用ログ](logging.md) とは別の種類として扱う。

## 1件の形

アプリは、所持データを変えたところで `GameLog` を作って渡す。数量の変化と、数量の変化でないものとで作り方を分ける。

| 作り方 | 使うもの | 例 |
|---|---|---|
| `GameLog::new(kind, delta, after, reason)` | 数量の増減。増減と更新後の値を持つ | ジェムの消費(`-300` と `2700`)、アイテムの付与 |
| `GameLog::event(kind, reason)` | 数量の変化でないもの。増減と更新後の値を持たない | キャラクターの入手、編成の変更、装備の付け替え、対戦の結果 |

どちらにも、次を足せる。

- `.target(id)`: 対象の ID(アイテムの ID など)。名前は変わることがあるので、ID を書く
- `.field(name, value)`: 名前付きの値(個体の ID・種類の ID・枠・前後の ID など)。値は数・文字列・配列など JSON にできるもの(`serde_json::Value` にできるもの)

| 値 | 渡す人 | 内容 |
|---|---|---|
| 種別・理由 | アプリ | 例: `gems`・`unit` と `gacha_draw`・`sign_up` |
| 対象の ID・増減と更新後の値・名前付きの値 | アプリ(どれも任意) | 上のとおり |
| `player_id` | 基盤(`run` の外では呼び出し側) | `AuthLayer` が入れた `AuthContext` から取る |
| `request_id`・処理の名前・時刻 | 基盤(`run` の外では呼び出し側) | `x-request-id`(形が正しいときだけ)、`rpc.method` と同じ名前、書いた時刻 |

- 数量の変化でないものを、`delta`・`after` に種類の ID を入れるような読み替えで書かないように、`event` と名前付きの値を分けて用意した
- 名前付きの値の名前は、英小文字・数字・`_`・`.` の1〜64文字にする。基盤が付ける項目(`kind`・`reason`・`target_id`・`delta`・`after`・`player_id`・`request_id`・`rpc.method`・`log_type`、ログの1行の `timestamp`・`level` など)と同じ名前と、同じ名前の重なりは、書くときにエラー(INTERNAL)にする。どちらかの値が黙って消えないようにするため

## 書き方

### `Transactions::run` の中

`run` に渡す処理が受け取る `TxContext` の `write_game_log` で書く。

- 書いたゲームログは `run` の中にため、処理が成功してコミットするときだけ出力先へ書く。ロールバックしたとき、再送で保存済みのレスポンスを返したとき(処理を呼ばない)に残らないことを、型で保証するため
- `player_id` は、`run` の引数ではなく `AuthContext` から取る。サインアップ(`player_id` を渡さない `run`)の初期データの付与も残すため
- `NO_SIDE_EFFECTS` の RPC で書くと、INTERNAL にする。読み取り専用のトランザクションで所持データは変わらないので、書くのは宣言か実装の間違いのため
- `AuthLayer` を付けていないサービスで書いても INTERNAL にする(誰のログか分からないため)

### `Transactions::run` の外

サーバー間の内部 API(別のポートで、基盤の Layer を付けずに共有シークレットで認証するもの)や定期実行のジョブは、`run` を通らずに自分でトランザクションを張る。そこでは `GameLogs` にゲームログをため、`GameLogSink::commit` でコミットする。出力先の扱いは `run` と同じ。

```rust
let mut tx = pool.begin().await?;
// ... 勝者にジェムを付与するなど、所持データを変える ...
let mut logs = GameLogs::new("internal.v1.MatchService/ReportResult").with_request_id(&request_id);
logs.write(winner, GameLog::new("gems", 100, 3100, "match_win"))?;
logs.write(loser, GameLog::event("match", "match_result").field("match_id", match_id))?;
game_log_sink.commit(tx, logs).await?; // Database なら tx の中で書いてからコミット、Stdout ならコミットの後に出す
```

- **`player_id` は、`write` で1件ずつ渡す**。内部 API には `AuthContext` が無く、1回の処理で複数のプレイヤーの所持データが変わることもあるため
- **処理の名前(`game_logs.method`・`rpc.method`)は呼び出し側が決める**。gRPC の RPC なら `rpc.method` と同じ形(パッケージ・サービス・メソッド)にし、運用ログやほかのゲームログと同じ名前で探せるようにする。RPC でない処理は `job/SeasonReward` のように、RPC と見分けられる名前にする。`RpcMethodLayer` が無いこともあるので、基盤はリクエストから取らない
- `request_id` は、呼び出し元から受け取った ID があれば `with_request_id` で付ける。形が正しく(英数字と `-` の1〜64文字)なければ付けない。無ければ `NULL`
- `commit` を呼ばずにトランザクションを捨てる(ロールバックする)と、ゲームログも残らない。処理の名前が使えない形(空、ASCII でない、255 文字を超える)なら、`commit` はコミットせずにエラーを返す

## 出力先

出力先は `GameLogSink` で選び、`Transactions::new` に渡す(`run` の外では `GameLogSink::commit` を呼ぶ)。`LogConfig` に入れないのは、書くのが `voltech-grpc-transaction` で、ログのクレートからトランザクションのクレートへ依存の向きを逆にしないため。

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
| `method` | 処理の名前(例: `sandbox.v1.GachaService/Draw`) |
| `kind` | 種別 |
| `body` | `reason` と、あれば `target_id`・`delta`・`after`・名前付きの値を、一番上に並べた JSON(例: `{"reason":"edit_party","party_id":1,"unit_ids":["u-1","u-2"]}`)。無い値は入れない |
| `created_at` | 書いた時刻 |

- 問い合わせで、プレイヤーの履歴を時刻の順に引くので、`(player_id, created_at)` にインデックスを付ける

### 標準出力

target `voltech_grpc_log::game`、メッセージ `game log` で、1件を1行で出す。項目は `log_type`(`game`)・`player_id`・`request_id`・`rpc.method`・`kind`・`reason` と、あれば `target_id`・`delta`・`after`・名前付きの値。名前付きの値も、ほかの項目と同じく一番上に並べる。

- **`RUST_LOG` のフィルターで消えない**。運用ログのレベルの設定で、問い合わせ・補填に使う履歴が消えないようにするため
- 運用ログと同じ流れで送られるので、`log_type` で見分けて分けて保存する
- **コミットに成功してから出すので、直後にプロセスが落ちると残らない**(書いた業務データはコミット済み)。残らないのは多くても1回分で、DB の業務データとは食い違いうる。食い違いを許せないなら `Database` を選ぶ

## 実装で決めたこと

- `TxContext` は、トランザクションを `ctx.tx()`(`&mut sqlx::Transaction`)で渡す。今までの `tx` を `ctx.tx()` に置き換えれば、同じように使える
- `write_game_log`・`GameLogs::write`・`GameLogSink::commit` は `Result<_, Internal>` を返す。業務の本体の関数(`Result<_, Internal>`)でも、ハンドラー(`Result<_, Status>`)でも `?` で書ける
- `run` の中のゲームログも、`TxContext` が持つ `GameLogs` にため、コミットのときに `GameLogSink::commit` で書く。`run` の中と外で、出力先の扱いを1か所にするため
- tracing のログの項目名はコンパイル時に決まっていなければならないので、標準出力では名前付きの値を1つの項目(`voltech_grpc_log::GAME_LOG_FIELDS`)に JSON の文字列で入れ、`voltech-grpc-log` の JSON の形式が一番上の項目に展開する。テキストの形式では、その項目に JSON のまま出る
- sandbox では、所持データを変える関数(`player::service::create`・`consume_gems`・`add_item`・`increment_counter`)の中で書く。サインアップは、プレイヤーを作ったとき(作成済みでないとき)だけ書く。数量の変化でないものの見本として、無料ガチャを使ったこと(種別 `free_draw`、名前付きの値 `drawn_on` に使ったゲーム内の日付)を書く
