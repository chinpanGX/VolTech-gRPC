---
title: "フォルダ構成"
status: deferred
area: repo
related: [repo/current-plan, api/errors]
proto: []
updated: 2026-10-11
---

# フォルダ構成

> **当初の案で、いまの形ではない。** いまは基盤をパッケージとして配り、サンプルはこのリポジトリの sandbox に置く([運用](operations.md))。いまのフォルダは [コードの地図](../dev/code-map.md) にある。下は、テンプレートをサンプル(`grpc-foundation-samples`)が兼ね、基盤を別のパッケージのリポジトリにする案の形。`foundation/` も当初の案で、いまは機能ごとのクレート(`voltech-grpc-transaction` など)にしている

テンプレート(アプリを作るときのいつもの形に合わせ、一番上を `Client/`・`Server/`・`Shared/` に分ける):

```
grpc-foundation-samples/
├── Client/                         Unity プロジェクト(アプリのもの。テンプレートでは最小の雛形)
│   ├── Packages/manifest.json      パッケージを Git URL(タグ)で参照する
│   └── Assets/Generated/           .proto から生成した C#(既定の出力先。Wrappers/・Messages/・Grpc/ に分ける。[コード生成](../api/codegen.md))
├── Server/                         Cargo ワークスペース
│   ├── foundation/                 基盤のクレート(voltech-grpc-transaction など): AppError、with_transaction()、冪等性、差分・バッジ・通知のコレクター、
│   │                               ErrorCode(error_code.proto から生成)。サーバーの基盤(普段は触らない)
│   └── crates/
│       ├── proto/                  アプリの .proto から tonic-build で生成したコード
│       ├── <ドメイン>/              ドメインのクレート(サンプルでは player と gacha)
│       ├── api/                    gRPCのハンドラー
│       └── server/                 バイナリ: main.rs(Layer の順番、ルーター、設定の読み込み)
├── Shared/
│   ├── proto/                      error_code.proto(予約コード入り)、アプリの .proto(既定の場所)
│   └── docs/                       本書の移管先
├── buf.yaml / buf.gen.yaml
└── docker-compose.yml / Makefile
```

パッケージ:

```
voltech-grpc/
├── proto/                          device.proto、account_data.proto、カスタムオプションの定義(default_action・skip_auth)
├── unity/                          UPM パッケージ: クライアントの基盤
│   ├── Core/                       Unity に依存しない本体(ApiCore、エラー処理、反映処理、FakeCallInvoker、汎用のテスト)
│   ├── Auth/                       認証(デバイス登録・トークンの発行・アカウント情報の保存)。Unity に依存しない
│   ├── Unity/                      Unity 用の薄い層(YetAnotherHttpHandler、時計、保存先、VContainer の登録)
│   └── Analyzers/                  カスタムエラーハンドラーの Roslyn アナライザー
├── rust/
│   ├── auth/                       voltech-grpc-auth: device.proto のサービスの実装、AuthLayer、PlayerResolver の trait、
│   │                               テーブルのマイグレーション(sqlx の migrate!() で埋め込む)
│   └── status/                     voltech-grpc-status: 強制アップデート・メンテナンス・日付変更の Layer、
│                                   server_status.toml の読み込み、サーバー時刻と日付のレスポンスヘッダー
└── tools/                          ラッパー生成の protoc プラグイン(dotnet tool)
```

クレートの依存の向き(サンプルの場合。左が右に依存する):

| クレート | 依存先 |
|---|---|
| `server`(バイナリ) | `api`、`voltech-grpc-auth`、`voltech-grpc-status`、`voltech-grpc-transaction` など |
| `api` | `gacha`、`player`、`proto`(生成コード)、`voltech-grpc-transaction` など |
| `gacha` | `player`、`voltech-grpc-transaction` など |
| `player` | `voltech-grpc-transaction` など |
| `voltech-grpc-auth`・`voltech-grpc-status`(パッケージ) | テンプレートのクレートには依存しない |

- **`.proto` の置き場所と、生成コードの出力先は、アプリごとに自由に変えられる**ようにする。`Shared/proto/`・`Client/Assets/Generated/` は既定の場所。パスを書くのは `buf.yaml`・`buf.gen.yaml` と、Rust の生成(`build.rs`)の設定だけにし、コードには埋め込まない
- 基盤の改善を upstream から取り込むときに触るのは `Server/foundation/`(`voltech-grpc-transaction` など)だけなので、アプリの変更と衝突しにくい。`Client/` には基盤のコードが無く、更新は `manifest.json` のタグを上げるだけ
- 新しい機能は、`Server/crates/` にドメインのクレートを足し、`Server/crates/api` にハンドラーを足す
- `Client/`・`Server/` の雛形は、サーバーが起動し、デバイス登録とトークンの発行が通るところまでを用意する(サーバー側は実装済み。[当面の実装方針](current-plan.md))
- パッケージはテンプレートのクレートに依存しないので、エラーは `tonic::Status` を直接作って返す(予約エラーは `errors.toml` から生成した定数で持つ。[エラー](../api/errors.md))
