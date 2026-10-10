---
title: "環境構築とよく使うコマンド"
status: current
area: ops
sections: []
related: [ops/proto-management, ops/testing, repo/current-plan, api/codegen]
proto: []
code: [Makefile, scripts/proto-tools.sh, .env.example]
updated: 2026-10-10
---

# 環境構築とよく使うコマンド

このリポジトリ(`grpc-foundation-samples`)で、サーバーとクライアントを動かすまでの手順。フォルダの分け方は [当面の実装方針](../repo/current-plan.md) にある。

| フォルダ | 内容 |
|---|---|
| `Server/` | API サーバー(Rust の Cargo ワークスペース) |
| `Unity/UnityProject/` | Unity クライアント |
| `Shared/proto/` | アプリの `.proto` |
| `Shared/docs/wiki/` | 設計(この wiki) |
| `Tools/protoc-gen-voltech-grpc/` | ラッパーを生成する protoc プラグイン([コード生成](../api/codegen.md)) |

## 必須のツール

| ツール | バージョン | 使うところ |
|---|---|---|
| Docker Desktop | | MySQL(`make db-up`) |
| Rust(rustup) | stable | サーバーのビルド・テスト |
| Unity | 6000.3.17f1 | クライアント |
| .NET SDK | 10 で確認 | クライアントの書き方のチェック(`make client-lint`) |
| GNU Make | | `make` のコマンド |
| Git for Windows | | Windows では、`make` のコマンドを Git Bash で動かす |
| Python | 3 | wiki の生成と lint(`make wiki-gen`・`make wiki-lint`) |

- `make` は PowerShell からでも Git Bash からでも実行できる。Windows では、PowerShell から実行しても、`make` の中のコマンドは Git Bash で動く(`C:/Program Files/Git/bin/bash.exe`)。Git を別の場所に入れたときは、環境変数 `GIT_BASH` でパスを渡す
- サーバーのビルドに protoc は要らない(`.proto` は `build.rs` の中で Rust 製のコンパイラが読む)

## `.proto` の生成に使うツール

クライアントの C# は `buf generate` で生成し、生成したコードをコミットしている。生成に使うツールは、`make` が必要になったときに `.tools/bin` へ自動で入れる(手で入れたり、PATH を設定したりしなくてよい)。

```
make proto-gen
```

`Unity/UnityProject/Assets/Scripts/Api/Generated` の下に、種類ごと(`Wrappers`: 呼ぶときに使うラッパー、`Messages`: リクエスト・レスポンスの型、`Grpc`: gRPC クライアント)・名前空間ごとのフォルダで出力される。`.proto` を消したときは、生成済みの `.cs` と `.meta` を手で消す(`buf.gen.yaml` で `clean` を使っていないため)。

| ツール | バージョン | 内容 |
|---|---|---|
| buf | 1.73.0 | `.proto` の lint と、生成の実行 |
| protoc | Grpc.Tools 2.84.0 に入っているもの | C# のメッセージ型の生成 |
| grpc_csharp_plugin | Grpc.Tools 2.84.0 に入っているもの | C# の gRPC クライアントの生成 |

- ツールだけを入れるときは `make proto-tools`。入れ方は `scripts/proto-tools.sh` にある(Windows・macOS・Linux)
- バージョンを上げるときは `scripts/proto-tools.sh` を書き換える。Grpc.Tools のバージョンは、Unity の `Packages/manifest.json` の `org.nuget.grpc.net.client` と同じにする

## よく使うコマンド

リポジトリのルートで実行する。全部の一覧は `make help`。

| コマンド | 内容 |
|---|---|
| `make run` | MySQL を起動し、サーバーを起動する(`.env` を読む) |
| `make test` | プッシュ前のテスト一式(全クレートのテスト(MySQL を使う)・プラグインのテスト・wiki の確認) |
| `make fmt` / `make lint` | rustfmt / Clippy |
| `make proto-gen` | `.proto` から C# を生成する(プラグインも先にビルドする) |
| `make proto-plugin` / `make plugin-test` | protoc プラグインをビルドして `.tools/voltech-grpc` に入れる / プラグインの単体テスト |
| `make wiki-gen` | wiki の API の表を `.proto` のコメントから生成し直す([API: DeviceService](../api/device-service.md) など) |
| `make wiki-lint` | wiki のリンク切れ・frontmatter の抜け・生成し忘れを確かめる |

## クライアントとサーバーの接続を確かめる

1. `.env.example` を `.env` にコピーし、`make run` でサーバーを起動する(`localhost:50051`)
2. Unity の Test Runner(Window > General > Test Runner)の EditMode で、`Tests.EditMode` を実行する
