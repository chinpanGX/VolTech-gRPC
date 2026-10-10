# Changelog

## [Unreleased]

- grpc-foundation-samples(タグ `pre-split`)から基盤(Rust のクレート、Unity クライアント、protoc プラグイン、wiki)を切り出す
- UPM パッケージ名を `com.chinpangx.voltech.grpc`、名前空間を `VolTech.Client` / `VolTech.V1` にする
- Unity の認証と Unity 用の層を `VolTech.Client.Auth`・`VolTech.Client.Unity` のアセンブリに分け、`ProtoErrorPolicyProvider` を基盤に入れる
- パッケージの `package.json` に依存(UniTask・YetAnotherHttpHandler・gRPC の DLL)を書く。利用側は `scopedRegistries` を足すだけでよい
- 基盤の動作確認用の sandbox(`.proto`・サーバー・Unity プロジェクト)と CI を足す
