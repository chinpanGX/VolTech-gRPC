# Changelog

## [Unreleased]

## [0.4.0] - 2026-10-17

- ゲームログを `Transactions::run` の外からも書け、数量の変化でない値も持てるようにする(#5)
  - サーバー間の内部 API や定期実行のジョブのように `run` を通らない処理は、`GameLogs::new(処理の名前)` に `write(player_id, log)` でためて、`GameLogSink::commit(tx, logs)` でコミットする。`Database` は同じトランザクションで書き、`Stdout` はコミットの後に出す(`run` と同じ)
  - 数量の変化でないもの(キャラクターの入手・編成の変更など)は `GameLog::event(kind, reason)` で作る。名前付きの値は `GameLog::field(name, value)` で足し、`game_logs.body` の JSON と標準出力の項目に並ぶ
  - `game_logs.body` は、無い値(`target_id` の無いときの `null` など)を入れなくなる。`GameLog` は `Eq` を実装しなくなる
  - アプリ: `GameLog::new` と `TxContext::write_game_log` はそのまま使える

## [0.3.0] - 2026-10-16

- サーバーのログの仕組みとして `voltech-grpc-log` クレートを足す(#3)。`LogConfig` を渡して `init` で準備し、`RpcLogLayer` を全サービスの一番外側に付けると、RPC ごとの span(`request_id`・`rpc.method` など。フィルターを上げても消えない)と、RPC の終わりのアクセスログ(ステータス・業務エラーの reason・処理時間)が出る。JSON(既定)は span の値を一番上の項目に並べ、GCP 向けの項目名(`severity` など)も選べる
  - アプリ: `tower_http::TraceLayer` と `tracing_subscriber::fmt().init()` を、`voltech_grpc_log::init(&config)` と `.layer(RpcLogLayer::new(&config))` に置き換える。基盤は環境変数を読まないので、`LOG_FORMAT`・`RUST_LOG` などはアプリが読んで `LogConfig` に入れる(sandbox の `config.rs` が見本)
  - ハンドラーが panic したとき、接続を切る代わりに INTERNAL(`INTERNAL_ERROR`)を返す
  - `RpcMethod` は `voltech-grpc-log` に移す。`voltech_grpc_transaction::RpcMethod` のままでも使える
- RPC の span に、認証したプレイヤー(`player_id`。`AuthLayer`・`DeviceService` が書く)と、再送で保存済みのレスポンスを返したか(`replayed`。`Transactions::run` が書く)が入り、アクセスログにも付く。デバイス登録・トークンの発行・認証の失敗・`x-request-id` の別の RPC での使い回し・メンテナンスの開始と終了をログに出す(トークンと `secret_key` は出さない)
- 内部エラーの原因を運ぶ `voltech_grpc_error::Internal` を足す(#3)。`std::error::Error` を実装したエラーから `?` で作れ、`Status` にするときに INTERNAL にして、原因を最後までたどった連なりと発生した場所をログに出す。業務エラーの `Status` を `?` で包んでも、`Status` に戻すときは元のまま返る。`result.or_internal()`(INTERNAL の `Status` に)と `result.context("...")`(説明を付ける)も使える
  - アプリ(壊れる変更): `internal()` は文字列を受け取らない。`internal("...")` は `internal(Internal::msg("..."))` か `Internal::msg("...").into()` に、`internal(format!(...))` は `Internal::msg(format!(...))` に、`Box<dyn Error>` は `Internal::from_boxed(err)` に替える。`.map_err(internal)` はそのまま使えるが発生した場所が残らないので、`.or_internal()` に替えるのをすすめる
  - 内部エラーのログの項目名は `err` から `exception.message` に変わり、`code.file.path`・`code.line.number` が付く
- ゲームログ(所持データの変化の履歴)を足す(#3)。`Transactions::run` の処理が受け取る `TxContext` の `write_game_log(GameLog::new(kind, delta, after, reason))` で書き、コミットしたときだけ残る(ロールバック・再送で保存済みのレスポンスを返したときは残らない)。出力先は `GameLogSink::Database`(`game_logs` テーブル)か `GameLogSink::Stdout`(標準出力の JSON。`RUST_LOG` で消えない)。`NO_SIDE_EFFECTS` の RPC で書くと INTERNAL
  - アプリ(壊れる変更): `Transactions::new(pool, levels)` は `Transactions::new(pool, levels, GameLogSink::Database)` に替える(出力先の環境変数の名前のおすすめは `GAME_LOG_SINK`)
  - アプリ(壊れる変更): `run` に渡す処理の引数が `&mut Transaction` から `&mut TxContext` になる。`move |tx| ...` は `move |ctx| ...` に、その中の `tx` は `ctx.tx()` に、`&mut **tx` は `&mut **ctx.tx()` に替える
  - アプリ: `voltech_grpc_transaction::migrate` が `game_logs` テーブルを作る。起動時のマイグレーションでそのまま作られる

## [0.2.0] - 2026-10-11

- `Google.Protobuf` を 3.36.2 に上げ、`Google.Api.CommonProtos` への依存を外す。Unity 6000.6 以降のエディタの中で、Google.Protobuf を CommonProtos の参照する版より上げると、業務エラーがすべて `CLIENT_ERROR` になっていたのを直す(#1)。エラーの詳細(`google.rpc.Status` / `ErrorInfo`)は `RpcErrorDetails` が自前で読む
  - Unity: `RpcErrorDetails.GetErrorInfo` は `Google.Rpc.ErrorInfo` の代わりに `RpcErrorInfo`(`Reason`・`Domain`・`Metadata`)を返す。タグを上げたら Unity でプロジェクトを開き直してパッケージを解決し直す。アプリが `Google.Rpc` の型を使っていたら、`org.nuget.google.api.commonprotos` を自分で足すか `RpcErrorInfo` に替える(足すときは、参照する Google.Protobuf の版がパッケージと同じものにする)。テストの asmdef の `precompiledReferences` から `Google.Api.CommonProtos.dll` を消す
  - protoc プラグインの `Google.Protobuf` も 3.36.2 に上げる
- protoc プラグインの `Tomlyn` を 2.10.1 に上げる(`errors.toml` は TOML 1.1 として読む。いまの書き方のままで読める)
- Rust の `Cargo.lock` を更新する(`serde_json` 1.0.152。`Cargo.toml` の要求の範囲内の更新だけで、アプリがすることは無い)

## [0.1.0] - 2026-10-11

- grpc-foundation-samples(タグ `pre-split`)から基盤(Rust のクレート、Unity クライアント、protoc プラグイン、wiki)を切り出す
- UPM パッケージ名を `com.chinpangx.voltech.grpc`、名前空間を `VolTech.Client` / `VolTech.V1` にする
- Unity の認証と Unity 用の層を `VolTech.Client.Auth`・`VolTech.Client.Unity` のアセンブリに分け、`ProtoErrorPolicyProvider` を基盤に入れる
- パッケージの `package.json` に依存(UniTask・YetAnotherHttpHandler・gRPC の DLL)を書く。利用側は `scopedRegistries` を足すだけでよい
- 基盤の動作確認用の sandbox(`.proto`・サーバー・Unity プロジェクト)と CI を足す
- `Google.Protobuf` を 3.31.1 にし、`Google.Api.CommonProtos` の依存する版にそろえる。Unity 6000.6 のエディタの中で、業務エラーがすべて不明なエラーになっていたのを直す
- Rust の `protox` を 0.10 に上げる(入れ子の深さが 32 までに制限される。MSRV 1.85)
- protoc プラグインのテストの `Microsoft.NET.Test.Sdk` を 18.10.1、`xunit.runner.visualstudio` を 4.0.1 に上げる
- 直前のレスポンスの行が無いプレイヤーへの最初の冪等性の対象のリクエストが、同時に届いた別のプレイヤーのリクエストとデッドロックし、INTERNAL になることがあったのを直す
- エラーコードを `.proto` の enum(`ErrorCode`・`default_action`)から、domain ごとの `errors.toml` に変える。エラーは `ErrorInfo` の domain と reason の組で見分け、基盤の予約エラーは domain `voltech` になる。reason から `ERROR_CODE_` の前置きを外す
  - アプリ: `error_code.proto` を `errors.toml`(アプリの domain)に書き換え、予約エラーは書かない。Rust は `build.rs` で `voltech_grpc_error_build::compile` を呼び、C# は `protoc-gen-voltech-grpc errors` で生成する
  - Rust: `failed_precondition` などは `ErrorCode` を受け取る。`voltech_grpc_error::code` は `reserved` になる
  - Unity: `ApiError.Code` が `ErrorCode`(domain と reason)になる。`ProtoErrorPolicyProvider` の代わりに `ErrorPolicyProvider`(生成した一覧を渡す)を使い、`ReservedErrorCodes` は `ReservedErrors` になる。`handledCodes` は `ErrorCode` で渡す
  - `voltech/v1/error_options.proto` を消す
- ガチャのサンプルを grpc-foundation-samples から sandbox に移す(サーバー・Unity の画面とシーン・テスト)
- エディタの通信を見るウィンドウを Request Viewer から gRPC Monitor に改名する(メニューは `Window > VolTech gRPC > gRPC Monitor`)。一覧を4列(Time・API・Result・Duration)にし、開いたときに広げる
- gRPC Monitor の記録を軽くする。Body を JSON にするのを、通信の後(ウィンドウに出す・実機から送る)まで遅らせ、通信を遅らせない。実機は結果を埋めた記録の Request を省いて送り、Body は字下げせずに送る(ウィンドウが表示するときに字下げする)。ウィンドウは、記録が埋まったときに詳細の Request を作り直さず、Follow latest でないときは一覧をスクロールしない
- MIT ライセンスで公開する(`LICENSE`。UPM パッケージにも `LICENSE.md` を入れる)
- アプリが C# を生成するための `mk/codegen.mk` を用意する。アプリの Makefile は、同じタグの基盤から生成に使うフォルダだけを取ってきて読み込む。使う `dotnet` などの設定は `local.mk` で上書きできる
