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
