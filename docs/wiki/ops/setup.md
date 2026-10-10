---
title: "環境構築とよく使うコマンド"
status: current
area: ops
related: [ops/proto-management, ops/testing, guide/getting-started, api/codegen]
proto: []
code: [Makefile, scripts/proto-tools.sh, .env.example, buf.gen.yaml, buf.gen.sandbox.yaml]
updated: 2026-10-11
---

# 環境構築とよく使うコマンド

このリポジトリ(VolTech gRPC)で、基盤を直して sandbox で確かめるまでの手順とコマンド。アプリから基盤を使う手順は [始め方](../guide/getting-started.md) にある。

| フォルダ | 内容 |
|---|---|
| `server/rust/` | サーバーの基盤クレート(Cargo ワークスペース) |
| `Unity/com.chinpangx.voltech.grpc/` | Unity クライアントの UPM パッケージ |
| `tools/protoc-gen-voltech-grpc/` | ラッパーを生成する protoc プラグイン([コード生成](../api/codegen.md)) |
| `sandbox/` | 基盤の動作確認とガチャのサンプル(`.proto`・サーバー・Unity プロジェクト。配らない) |
| `docs/wiki/` | 設計(この wiki) |

## 必須のツール

| ツール | バージョン | 使うところ |
|---|---|---|
| Docker Desktop | | MySQL(`make db-up`) |
| Rust(rustup) | stable(edition 2024) | サーバーのビルド・テスト |
| Unity | 6000.6.5f1 | sandbox の Unity プロジェクト(パッケージのコンパイルとテスト) |
| .NET SDK | 10 で確認 | protoc プラグインと、クライアントの書き方のチェック(`make client-lint`) |
| GNU Make | | `make` のコマンド |
| Git for Windows | | Windows では、`make` のコマンドを Git Bash で動かす |
| Python | 3.11 以上 | wiki の生成と lint(`make wiki-gen`・`make wiki-lint`)、依存の確認(`make deps-check`・`make deps-outdated`) |

- `make` は PowerShell からでも Git Bash からでも実行できる。Windows では、PowerShell から実行しても、`make` の中のコマンドは Git Bash で動く(`C:/Program Files/Git/bin/bash.exe`)。Git を別の場所に入れたときは、環境変数 `GIT_BASH` でパスを渡す
- サーバーのビルドに protoc は要らない(`.proto` は `build.rs` の中で Rust 製のコンパイラが読む)

## `.proto` の生成に使うツール

クライアントの C# は `buf generate` で生成し、生成したコードをコミットしている。生成に使うツールは、`make` が必要になったときに `.tools/bin` へ自動で入れる(手で入れたり、PATH を設定したりしなくてよい)。

```
make proto-gen           # 基盤の .proto → Unity/com.chinpangx.voltech.grpc/Runtime/Generated
make sandbox-proto-gen   # sandbox の .proto → sandbox/UnityProject/Assets/Generated(ラッパーを含む)
```

種類ごと(`Wrappers`: 呼ぶときに使うラッパー、`Messages`: リクエスト・レスポンスの型、`Grpc`: gRPC クライアント)・名前空間ごとのフォルダで出力される。基盤のデバイス登録は Auth が直接呼ぶので、基盤の分のラッパーは生成しない。`.proto` を消したときは、生成済みの `.cs` と `.meta` を手で消す(`clean` を使っていないため)。

| ツール | バージョン | 内容 |
|---|---|---|
| buf | 1.73.0 | `.proto` の lint と、生成の実行 |
| protoc | Grpc.Tools 2.84.0 に入っているもの | C# のメッセージ型の生成 |
| grpc_csharp_plugin | Grpc.Tools 2.84.0 に入っているもの | C# の gRPC クライアントの生成 |

- ツールだけを入れるときは `make proto-tools`。入れ方は `scripts/proto-tools.sh` にある(Windows・macOS・Linux)
- バージョンを上げるときは `scripts/proto-tools.sh` を書き換える。Grpc.Tools のバージョンは、パッケージの `package.json` の `org.nuget.grpc.net.client` と同じにする

## よく使うコマンド

リポジトリのルートで実行する。全部の一覧は `make help`。

| コマンド | 内容 |
|---|---|
| `make sandbox-server` | MySQL を起動し、sandbox のサーバーを起動する(`.env` を読む。DB は `voltech_sandbox`) |
| `make test` | プッシュ前のテスト一式(基盤と sandbox のクレートのテスト(MySQL を使う)・プラグインのテスト・wiki の確認・依存の組み合わせの確認) |
| `make fmt` / `make lint` | rustfmt / Clippy(基盤と sandbox の両方) |
| `make proto-gen` / `make sandbox-proto-gen` | `.proto` から C# を生成する |
| `make proto-plugin` / `make plugin-test` | protoc プラグインをビルドして `.tools/voltech-grpc` に入れる / プラグインの単体テスト |
| `make wiki-gen` | wiki の API の表を `.proto` のコメントから生成し直す([API: DeviceService](../api/device-service.md) など) |
| `make wiki-lint` | wiki のリンク切れ・frontmatter の抜け・生成し忘れを確かめる |
| `make deps-check` / `make deps-outdated` | 一緒に上げないと壊れる依存の組み合わせを確かめる / 依存の新しい版を調べる(上げる手順はスキル `/deps-update`) |

## クライアントとサーバーの接続を確かめる

1. `.env.example` を `.env` にコピーし、`make sandbox-server` でサーバーを起動する(`localhost:50051`)
2. Unity で `sandbox/UnityProject` を開く。パッケージは `file:` で参照しているので、パッケージのコードを直すとそのまま反映される
3. Test Runner(Window > General > Test Runner)の EditMode で、すべてを実行する。パッケージのテスト(`VolTech.Client.Tests.EditMode`。`testables` で出している)と sandbox のテスト(`Sandbox.Tests.EditMode`)が並ぶ。`SandboxServerTests` が起動したサーバーにつなぐ

コマンドラインからは `unity test sandbox/UnityProject --mode EditMode` でも流せる(Unity CLI)。
