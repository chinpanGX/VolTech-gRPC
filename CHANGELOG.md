# Changelog

## [Unreleased]

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
