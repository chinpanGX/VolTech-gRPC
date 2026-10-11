# VolTech gRPC

gRPC でゲームのバックエンドを作るための基盤。Rust のサーバー側クレート、Unity のクライアント(UPM パッケージ)、C# のラッパーを生成する protoc プラグインからなる。アプリは自分のプロジェクトから、タグを指定して参照する。

| 場所 | 中身 |
| --- | --- |
| `server/rust/` | サーバーの基盤クレート(`voltech-grpc-auth`・`-error`・`-error-build`・`-status`・`-transaction`) |
| `Unity/com.chinpangx.voltech.grpc/` | Unity クライアントの UPM パッケージ |
| `tools/protoc-gen-voltech-grpc/` | ApiCore を通して呼ぶ C# ラッパーを生成する protoc プラグイン |
| `sandbox/` | 基盤の動作確認とガチャのサンプル(proto・サーバー・Unity プロジェクト)。組み込み方の実例 |
| `docs/` | 設計と利用ガイド(`docs/wiki/`) |

sandbox は配布物ではない。基盤の動作確認と、組み込み方の実例のためにある。

## 使い方

[docs/wiki/guide/getting-started.md](docs/wiki/guide/getting-started.md)。

## 開発

`make help` でコマンドの一覧が出る。プッシュ前は `make test`。セットアップは [docs/wiki/appendix/setup.md](docs/wiki/appendix/setup.md)。

## ライセンス

MIT。[LICENSE](LICENSE)。
