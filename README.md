# VolTech gRPC

gRPC でゲームのバックエンドを作るための基盤。Rust のサーバー側クレート、Unity のクライアント(UPM パッケージ)、C# のラッパーを生成する protoc プラグインからなる。

| 場所 | 中身 |
| --- | --- |
| `server/rust/` | サーバーの基盤クレート(`voltech-grpc-auth`・`-error`・`-status`・`-transaction`) |
| `Unity/com.chinpangx.voltech.grpc/` | Unity クライアントの UPM パッケージ |
| `tools/protoc-gen-voltech-grpc/` | ApiCore を通して呼ぶ C# ラッパーを生成する protoc プラグイン |
| `sandbox/` | 基盤の動作確認用の proto・サーバー・Unity プロジェクト |
| `docs/` | 設計と利用ガイド(`docs/wiki/`) |

sandbox は配布物ではない。基盤の動作確認・改修の検証用で、業務機能は入れない。

## 使い方

- Rust: `voltech-grpc-* = { git = "https://github.com/chinpanGX/VolTech-gRPC", tag = "v0.1.0" }`
- Unity: `Packages/manifest.json` に `"com.chinpangx.voltech.grpc": "https://github.com/chinpanGX/VolTech-gRPC.git?path=/Unity/com.chinpangx.voltech.grpc#v0.1.0"`

詳しくは [docs/wiki/guide/getting-started.md](docs/wiki/guide/getting-started.md)。

## 開発

`make help` でコマンドの一覧が出る。プッシュ前は `make test`。セットアップは [docs/wiki/ops/setup.md](docs/wiki/ops/setup.md)。
