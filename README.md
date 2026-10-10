# VolTech gRPC

gRPC でゲームのバックエンドを作るための基盤とテンプレート。Rust のサーバー側クレート、Unity のクライアント(UPM パッケージ)、C# のラッパーを生成する protoc プラグインと、それを使うアプリの雛形(`sandbox/`)からなる。アプリはこのリポジトリを clone して作る。

| 場所 | 中身 |
| --- | --- |
| `server/rust/` | サーバーの基盤クレート(`voltech-grpc-auth`・`-error`・`-status`・`-transaction`) |
| `Unity/com.chinpangx.voltech.grpc/` | Unity クライアントの UPM パッケージ |
| `tools/protoc-gen-voltech-grpc/` | ApiCore を通して呼ぶ C# ラッパーを生成する protoc プラグイン |
| `sandbox/` | アプリの雛形(proto・サーバー・Unity プロジェクト)。基盤の動作確認にも使う |
| `docs/` | 設計と利用ガイド(`docs/wiki/`) |

このリポジトリの sandbox には業務機能を入れない(アプリは clone した先で sandbox を書き換える)。

## 使い方

[docs/wiki/guide/getting-started.md](docs/wiki/guide/getting-started.md)。既存のアプリからタグで参照する形は [docs/wiki/repo/operations.md](docs/wiki/repo/operations.md)。

## 開発

`make help` でコマンドの一覧が出る。プッシュ前は `make test`。セットアップは [docs/wiki/ops/setup.md](docs/wiki/ops/setup.md)。
