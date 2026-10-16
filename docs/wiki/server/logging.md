---
title: "運用ログ"
status: current
area: server
related: [server/request-flow, server/game-log, server/transaction, server/auth, server/status, guide/server-reference, guide/debugging, api/errors, api/metadata]
proto: []
code: [server/rust/log, server/rust/error/src/internal.rs, server/rust/auth/src/layer.rs, server/rust/auth/src/device_service.rs, server/rust/status/src/status.rs, sandbox/server/server/src/config.rs, sandbox/server/server/src/main.rs]
updated: 2026-10-17
---

# 運用ログ

障害や問い合わせのときに「誰の・どのリクエストで・何が・なぜ起きたか」をたどれるように、ログの出し方を基盤の `voltech-grpc-log` クレートにまとめる。アプリは設定を渡して初期化し、`RpcLogLayer` を付けるだけにする。

## 設定

基盤のほかのクレートと同じく、基盤は環境変数を読まず、設定の構造体 `LogConfig` を受け取る。アプリが環境変数などから読んで渡す(環境変数の名前のおすすめは [サーバーのリファレンス](../guide/server-reference.md))。

| 項目 | 内容 | 既定 |
|---|---|---|
| `format` | `LogFormat::Json`(1行1件の JSON)か `LogFormat::Text`。どちらも標準出力に出す | `Json` |
| `filter` | `RUST_LOG` の形式のフィルター | `info` |
| `field_names` | JSON の項目名。`FieldNames::Standard`(`timestamp`・`level`)か `FieldNames::Gcp`(`time`・`severity`) | `Standard` |
| `service_name`・`environment` | どの行にも付ける `service.name`・`deployment.environment.name`。複数のサーバーのログを同じ送り先に集めたときに見分けるため | `LogConfig::new` の引数 |
| `excluded_rpcs` | アクセスログから除く RPC。サービス名か、メソッドまで含めた名前で書く | ヘルスチェックとリフレクション(`DEFAULT_EXCLUDED_RPCS`)。数秒おきに呼ばれてログを埋めるため |

- 時刻は UTC の RFC 3339(ミリ秒まで。例: `2026-10-16T01:02:03.456Z`)に固定し、設定にしない。送り先ごとに時刻の解釈が割れないようにするため
- テキストは、標準出力がターミナルで、`NO_COLOR` が無いときだけ色を付ける
- 知らない値(形式の名前の書き間違い、フィルターの書き間違い)は、文字列から読むとき(`FromStr`)と初期化のときにエラーにする。気づかないまま別の形式で動き続けないようにするため

## RPC ごとの span

一番外側の `RpcLogLayer` が、RPC ごとに span(target `voltech_grpc_log::rpc`)を作る。span の中で出たログには、次の項目が自動で付く。

| 項目 | 内容 |
|---|---|
| `rpc.system.name` | `grpc` |
| `rpc.method` | サービス名まで含めた名前(例: `app.v1.GachaService/Draw`)。gRPC のパスの形でなければ `_OTHER` |
| `request_id` | `x-request-id`。形が正しい(英数字と `-` の1〜64文字)ときだけ入る |
| `player_id` | 認証したプレイヤー。`AuthLayer` が書く。認証の無い `DeviceService` では、`Register` が発行した ID を、`IssueToken` がデバイスの ID を書く |
| `replayed` | 冪等性の対象の RPC で、再送で保存済みのレスポンスを返したら `true`、処理を動かしたら `false`。`Transactions::run` が書く。対象でない RPC には付かない |

- `player_id`・`replayed` は、span を作るときは空にしておき、RPC の途中で `voltech_grpc_log::field` の名前で書き込む。書き込んだ後に出たログ(アクセスログを含む)に付く。値を読み返す仕組みは作らず、formatter が span の値を付ける
- 項目名は OpenTelemetry の RPC の semantic conventions(Release Candidate、2026-10 時点)に合わせる。後でトレースを送るときに、そのまま使えるようにするため
- **RPC の span は、フィルターのレベルを上げても有効にする**。`RUST_LOG=warn` で span が無効になると、warn・error のログに `request_id` が付かず、いちばん追いたいログがたどれなくなるため
- メタデータは `x-request-id` しか読まない。`authorization`(トークン)やリクエストの中身はログに出さない

## アクセスログ

RPC の終わりに、span の中で1行だけ出す(target `voltech_grpc_log::access`、メッセージ `rpc finished`)。

| 項目 | 内容 |
|---|---|
| `rpc.response.status_code` | gRPC のステータスの名前(例: `OK`・`FAILED_PRECONDITION`) |
| `error.type` | サーバー側のエラー(`UNKNOWN`・`DEADLINE_EXCEEDED`・`UNIMPLEMENTED`・`INTERNAL`・`UNAVAILABLE`・`DATA_LOSS`)のときだけ、ステータスの名前 |
| `error.domain`・`error.reason` | エラーの `ErrorInfo` の domain と reason |
| `duration_ms` | 受け取ってから、レスポンスの body を送り終わるまでの時間(ミリ秒) |
| `panic` | ハンドラーが panic したときだけ `true` |

- ステータスは、レスポンスの body をラップして取る。成功したときは body の後ろの trailers に、Trailers-Only のエラーは headers に入っているため
- reason は、エラーを作った場所ではなく、返したレスポンスの `grpc-status-details-bin` から読む。作ったエラーが返るとは限らないため。この読み方で、メンテナンスや強制アップデートなど Layer が返したエラーの reason も取れる
- **業務エラーは error にしない**。レベルは、`UNKNOWN`・`INTERNAL`・`DATA_LOSS` と panic のときだけ error、ほかは info。業務エラーで error が増えると、サーバーの不具合を見落とすため
- **必ず1行出す**。レスポンスを返す前に future が捨てられたとき(クライアントの切断・期限切れ)は、捨てられたときに `CANCELLED` で出す
- ハンドラーの panic は `RpcLogLayer` が捕まえ、`internal()` でログに出して INTERNAL(`INTERNAL_ERROR`)を返す。tokio はタスクの panic を捕まえてから future を捨てるので、捨てられたときには panic と切断を見分けられないため。クライアントには、接続が切れる代わりに、ほかの内部エラーと同じエラーが返る

## 内部エラー

INTERNAL を返すときは、`voltech_grpc_error::Internal` で原因を運び、`Status` にするときに1行 error で出す(メッセージ `internal error`)。

| 項目 | 内容 |
|---|---|
| `exception.message` | 処理の説明(あれば)と、元のエラーから `source()` を最後までたどった原因を `: ` でつないだもの(例: `grant item: query failed: connection reset`) |
| `code.file.path`・`code.line.number` | 発生した場所。`?`・`internal()`・`or_internal()`・`Internal::msg` を書いた場所(`#[track_caller]`) |

- 原因を最後までたどる。`Display` だけでは、`sqlx` などのエラーの奥にある原因(接続が切れた、など)が残らないため
- `.map_err(internal)` のように関数を渡すと、発生した場所が標準ライブラリの中になる。場所を残したいときは `or_internal()` を使う
- `Internal` は `std::error::Error` を実装しない。実装すると、`?` で作るための `From<E: Error>` と標準の `From<T> for T` がぶつかるため(anyhow と同じ制約)
- `tonic::Status` も `std::error::Error` を実装しているので、業務エラーに `?` を使うと `Internal` に包まれる。黙って INTERNAL にならないように、`Status` に戻すときに中身が `Status` なら元のまま返す(error のログも出さない)

## 基盤の出来事

基盤が出すログと、そのレベル。RPC の中で出るものには、span の値(`request_id`・`player_id` など)が付く。

| 出来事 | メッセージ | レベル | 項目 |
|---|---|---|---|
| RPC の終わり | `rpc finished` | info(サーバーのエラー・panic は error) | 上の「アクセスログ」 |
| 内部エラー | `internal error` | error | 上の「内部エラー」 |
| デバイス登録 | `device registered` | info | `device_id` |
| トークンの発行 | `access token issued` | info | `device_id`。トークンは出さない |
| トークンの発行の失敗 | `access token issue failed` | warn | `reason`(`unknown_device`・`invalid_secret`)、`device_id` |
| 認証の失敗: トークンが無い | `authentication failed` | warn | `reason` = `missing_token` |
| 認証の失敗: 期限切れのトークン | `authentication failed` | info | `reason` = `expired_token`、`device_id` |
| 認証の失敗: 知らないトークン | `authentication failed` | info | `reason` = `unknown_token` |
| `x-request-id` の別の RPC での使い回し | `x-request-id was already used for another RPC` | warn | `saved_method`(先に使った RPC) |
| メンテナンスの開始・終了(設定の読み直しで切り替わったとき) | `maintenance started`・`maintenance ended` | info | `end_at`(開始のとき、終了予定があれば) |
| サーバーの状態の読み直しの失敗 | `failed to reload server status; keep the previous one` | warn | `err` |

- warn は「放っておくと困るかもしれないので見る」もの。期限切れのトークンと知らないトークンは、クライアントがトークンを発行し直す前後の普段の動作でも起きる(発行し直すと古いトークンの行は上書きされ、知らないトークンになる)ので warn にしない
- トークンが無い・発行の失敗は、普段の動作では起きない(クライアントの不具合か、不正なリクエスト)ので warn にする
- `authorization`・トークン・`secret_key`・リクエストの中身は、どのログにも出さない

## 送り先ごとの注意

- Cloud Logging に送るときは `FieldNames::Gcp` にする。レベルは `severity`(warn は `WARNING`)、時刻は `time` になり、Cloud Logging がそのまま読む
- Cloud Logging から BigQuery へ送ると、項目名の `.` が `_` に変わる(例: `rpc.method` は `rpc_method`)。BigQuery で集計するクエリは、変わった後の名前で書く
- ゲームログを標準出力に出すときは、運用ログと同じ流れで送られる。`log_type` が `game` の行だけを分けて保存する([ゲームログ](game-log.md))

## 実装で決めたこと

- **JSON は自前で書く**(`FormatEvent` ではなく tracing-subscriber の `Layer`)。tracing-subscriber の JSON は span の値を `span`・`spans` の下に入れ子にするので、`request_id` などで検索・集計しにくい。span の値は作ったときと `record` のときに span の extensions にため、ログを出すときに外側の span から順に一番上の項目へ並べる(同じ名前は内側の値が勝つ)
- 基盤が付ける項目(時刻・レベル・`message`・`target`・`service.name`・`deployment.environment.name`)は、同じ名前の値をログに書いても上書きされない
- GCP 向けの severity は Cloud Logging の名前に合わせ、warn を `WARNING` にする。TRACE に当たるものは無いので `DEBUG` にする
- 1行は1回の書き込みで出し、ほかのスレッドの行と混ざらないようにする
- 初期化は、プロセス全体の出力先にする `init` と、組み合わせられる Layer を返す `layer` の2つ。OpenTelemetry などの Layer を後から足せるようにするため。テストでは、出力先を差し替える `layer_with_writer` を使う
- RPC の名前(`RpcMethod`)と `x-request-id` の形の確認(`is_valid_request_id`)は、このクレートに置き、`voltech-grpc-transaction` と共有する。パスの解析と形の確認を1か所にするため
- 認証の失敗の理由(期限切れか、知らないトークンか)を見分けるため、トークンの期限は SQL の条件ではなく値として読み、期限切れを判定する
- ゲームログの target(`voltech_grpc_log::game`)のログは、運用ログのフィルター(`RUST_LOG`)にかかわらず出す。問い合わせ・補填に使う履歴が、レベルの設定で消えないようにするため
- ゲームログの名前付きの値は、名前が実行時に決まり tracing の項目にできないので、`GAME_LOG_FIELDS`(`game.fields`)に JSON の文字列で受け取り、JSON の形式では一番上の項目に展開する。基盤が付ける項目は上書きしない。ゲームログ以外のログでは展開しない([ゲームログ](game-log.md))
