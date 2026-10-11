---
title: "コード生成"
status: confirmed
area: api
related: [repo/folder-layout, api/errors, api/idempotency, api/auth, server/transaction, server/auth, client/wrapper, client/response-items, client/fake-testing]
proto: []
code: [tools/protoc-gen-voltech-grpc, buf.gen.yaml]
updated: 2026-10-11
---

# コード生成

```
*.proto(APIの唯一の定義元)
 ├─ tonic-build (build.rs) ─────────→ Rust: サービスtrait + メッセージ型
 ├─ protoc + grpc_csharp_plugin ────→ C#: メッセージ型 + クライアント
 └─ 自作のprotocプラグイン(C#) ─────→ C#: ラッパー、全RPCの一覧、共通レスポンス項目の一覧
```

- **C#の[ラッパー](../client/wrapper.md)は、自作のprotocプラグインで生成する**。`buf generate` で protoc組み込みのC#生成・grpc_csharp_plugin と一緒に実行する。Unity では Grpc.Tools が動かないので、**生成したC#コードはコミットする**
- プラグインは **C#** で書く(入出力の型 `CodeGeneratorRequest` は `Google.Protobuf` に入っている。クライアント側の道具と言語がそろう)。基盤のリポジトリ(VolTech-gRPC)に置き、パッケージとしては配らない。アプリは同じタグの基盤を取ってきてビルドする([始め方](../guide/getting-started.md))。生成するラッパーは `ApiCore` の API に合わせるので、`ApiCore` と同じタグで使う
- `.proto` の確認と C# の生成には **buf** を使う。`buf lint`(書き方)・`buf format`(整形)・`buf breaking`(後方互換を壊す変更の検出)・`buf generate`(C# の生成)。Rust は `build.rs` で生成する(protox で `.proto` を読み込み、tonic-prost-build で生成する。protoc は要らない)
- 入力の `.proto` の場所と、生成コードの出力先は、アプリごとに変えられる(`buf.yaml`・`buf.gen.yaml` と `build.rs` に書く。[始め方](../guide/getting-started.md))
- プラグインが生成するもの
  - ラッパー(gRPCサービスごとに1クラス)。RPCごとの `idempotency_level`([冪等性](idempotency.md))と、サービスごとの `skip_auth`([認証](auth.md))を読んで埋め込む
  - 全RPCの一覧(汎用のテスト用。[サーバーが無いときの差し替えとテスト](../client/fake-testing.md))
  - 共通レスポンス項目(`player_diff`・`badges`・`notifications`)を持つレスポンスの一覧(反映処理の登録漏れの確認用。[共通レスポンス項目の反映](../client/response-items.md))。それらのレスポンスの `partial class` に、目印のinterface(`IHasPlayerDiff`・`IHasBadges`・`IHasNotifications`)を付ける
  - `errors` サブコマンドで、エラーの定義ファイル(`errors.toml`)から C# のエラーコードの定数と、既定の action の一覧を作る([エラー](errors.md))
- **カスタムオプションは `skip_auth` だけ**にする
  - `skip_auth`(認証が不要なサービス。[認証](auth.md))
  - エラーの既定の action は `.proto` に書かず、`errors.toml` に書く([エラー](errors.md))
  - 冪等性は、標準のメソッドオプション `idempotency_level` で表す([冪等性](idempotency.md))

| オプション | クライアント | サーバー |
|---|---|---|
| `skip_auth` | ラッパーの生成時にプラグインが読む | v1では読まない([認証(サーバー)](../server/auth.md)) |
| `idempotency_level` | ラッパーの生成時にプラグインが読む | v1から読む(標準のフィールドなので `prost-types` で読める。[with_transaction() と冪等性](../server/transaction.md)) |

## 実装で決めたこと

- プラグインは `tools/protoc-gen-voltech-grpc/`(`src/` が本体、`tests/` が単体テスト)。`make proto-plugin` で `.tools/voltech-grpc` に入れ、`make proto-gen` の前に自動で入れ直す。テストは `make plugin-test`
- **なぜ生成にするか**: `.proto` の情報(RPC の名前、`idempotency_level`、`skip_auth`、共通レスポンス項目のフィールド名)を、実行時に Descriptor から読んだり手で書き写したりすると、タイポや食い違いがコンパイルで見つからない。生成すれば、`.proto` が唯一の定義元のまま、使う側は型と定数で書ける
- `.proto` のファイルごとに `<ファイル名>Api.cs` を1つ作る(`player.proto` → `App/V1/PlayerApi.cs`)。出力先のフォルダは、protoc 組み込みの C# 生成と同じく名前空間から決める。生成するものが無いファイルは作らない
- **生成先は、生成するものの種類でフォルダを分ける**(`Generated/Wrappers`・`Generated/Messages`・`Generated/Grpc`。その下は名前空間ごと)。同じフォルダに出すと、1つの `.proto` から `Player.cs`・`PlayerGrpc.cs`・`PlayerApi.cs` が並び、通信するときにどれを見ればよいか分かりにくいため。呼ぶときに見るのは `Wrappers`、フィールドを確かめるのは `Messages`、`Grpc` は `ApiCore` の中でしか使わない
- 名前空間は分けない(どれも `App.V1`)。ラッパーのファイルはメッセージ型の `partial class` に目印のインターフェースを付けるので、同じ名前空間でないとコンパイルできない。また、呼ぶ側はラッパーとメッセージ型を必ず両方使うので、`using` が1つで済む
- 1つのファイルに生成するもの
  - `<サービス名>Methods`: RPC のパスの定数(`PlayerServiceMethods.SignIn` = `/app.v1.PlayerService/SignIn`)。`RpcSettings` の名前と、テストの `FakeCallInvoker` への登録に使う
  - ラッパー: クラス名はサービス名の末尾の `Service` を `Api` に置き換える(`PlayerService` → `PlayerApi`)。`.proto` のサービスと RPC のコメントを doc コメントにする。名前空間はメッセージ型と同じ(`App.V1`)
  - 目印のインターフェース(`IHasPlayerDiff` など): フィールドの型を定義したファイルに出す。プロパティの型はそのフィールドの型
  - 共通レスポンス項目を持つメッセージの `partial class` に、目印のインターフェースを付ける
- オプション(`buf.gen.yaml` の `opt`)
  - `package=app.v1`: 生成するパッケージ。複数書ける。書かなければ全部。基盤のデバイス登録(`voltech.v1`)は Auth が直接呼ぶので、アプリのパッケージだけにする
- `skip_auth` は、アプリの `.proto` が `voltech/v1/auth_options.proto` を import しないので、番号(50002)で未知のフィールドから読む。プラグインと auth パッケージは同じタグで出すので、番号がずれることはない
- ストリーミングの RPC はエラーにする(`ApiCore` は単項の呼び出しだけを扱う)
- 1つのファイルに複数の型を出す(手書きのコードの「1ファイル1型」は、生成コードには当てはめない。protoc の生成と同じ)
