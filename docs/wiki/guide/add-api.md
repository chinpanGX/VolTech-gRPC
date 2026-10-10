---
title: "API を足す"
status: current
area: guide
related: [guide/client-reference, guide/server-reference, guide/tools, api/codegen, api/errors, api/idempotency, api/common-response]
proto: [sandbox/proto/sandbox/v1/sandbox.proto, sandbox/proto/sandbox/v1/error_code.proto]
code: [sandbox/server/server/src, sandbox/server/server/src/lib.rs, sandbox/server/proto/build.rs, buf.gen.sandbox.yaml]
updated: 2026-10-10
---

# API を足す

新しい API を足すときの手順。パスとコマンドは、テンプレートの雛形(`sandbox/`)の構成で書く([始め方](getting-started.md))。コードの例はサンプルのガチャ(`GachaService/Draw`。grpc-foundation-samples に全体がある)。`.proto` が API の唯一の定義元で、サーバー(Rust)とクライアント(C#)のコードはそこから生成する([コード生成](../api/codegen.md))。

## 1. `.proto` を書く

アプリの `.proto` は `sandbox/proto/sandbox/v1/` に置く。ファイルを足したら、サーバーの生成の対象(`sandbox/server/proto/build.rs` の `protox::compile` に渡す一覧)にも足す。

```proto
service GachaService {
  // ジェムを消費してガチャを1回引き、排出されたアイテムを1つ付与する。
  rpc Draw(DrawRequest) returns (DrawResponse);
}
```

- **RPC・メッセージ・フィールドの直前のコメントが、API の説明になる**。生成したラッパーの XML ドキュメントにも入る。フィールドには値の形(長さ・文字・単位)まで書く
- **冪等性**: 何も書かなければ「冪等性の対象」になる。クライアントは RequestID を付けて1つずつ送り、サーバーは同じ RequestID の再送で二重に実行しない。読むだけの RPC は `option idempotency_level = NO_SIDE_EFFECTS;`、何回実行しても結果が同じ RPC は `IDEMPOTENT` を書く([冪等性](../api/idempotency.md))
- **認証**: 基盤の `DeviceService` 以外は、すべて認証が要る(アプリが `skip_auth` を使うことは想定していない。[認証](../api/auth.md))
- **共通レスポンス項目**: プレイヤーのデータを変える RPC は、レスポンスに `PlayerDiff player_diff`・`repeated Badge badges`・`repeated Notification notifications` を、この名前で持つ。クライアントの反映処理が自動で読む([共通レスポンス項目](../api/common-response.md))
- **業務エラー**: 新しいエラーは `error_code.proto` の `ErrorCode` に、1000 以降の番号で足し、既定の action を付ける(`[(voltech.v1.default_action) = ERROR_ACTION_STAY]`)。コメントに、どの RPC がいつ返すかを書く([エラー](../api/errors.md))

書いたら `make proto-lint` で確かめる。

## 2. コードを生成する

```
make sandbox-proto-gen
```

- C#: `sandbox/UnityProject/Assets/Generated/` に、メッセージ型(`Messages/`)・gRPC クライアント(`Grpc/`)・ラッパー(`Wrappers/`。`GachaApi` のように、サービスごとに1クラス)ができる。生成したコードはコミットする
- Rust: 生成のコマンドは要らない。サーバーをビルドすると、`sandbox/server/proto` の `build.rs` が生成する(生成コードはコミットしない)

## 3. サーバーのハンドラーを書く

`sandbox/server/server/src/` に、生成されたサービスの trait を実装する(雛形では `sandbox_service.rs`)。本体は `Transactions::run` の中で動かす。トランザクションと、冪等性の対象の RPC の二重実行の防止を、基盤がまとめて行う。

```rust
async fn draw(&self, request: Request<DrawRequest>) -> Result<Response<DrawResponse>, Status> {
    let player_id = player_id(&request)?; // AuthLayer が入れた認証の情報から取る
    let response = self
        .transactions
        .run(&request, Some(player_id), move |tx| {
            Box::pin(async move {
                // tx で DB を読み書きする。業務エラーは failed_precondition で返す
                gacha::service::draw(tx, /* ... */).await.map_err(|err| match err {
                    DrawError::InsufficientGems => failed_precondition(
                        ErrorCode::InsufficientGems.as_str_name(),
                        "not enough gems",
                    ),
                    DrawError::Database(err) => internal(err),
                    // ...
                })?;
                Ok(DrawResponse { /* ... */ })
            })
        })
        .await?;
    Ok(Response::new(response))
}
```

作ったサービスは `sandbox/server/server/src/lib.rs` で登録する。認証が要るサービスには `AuthLayer` と `DateChangeLayer` を付ける(付け忘れると、ハンドラーで認証の情報が取れずに INTERNAL になる)。

```rust
.add_service(auth.layer(date_change.layer(GachaServiceImpl::new(transactions).into_server())))
```

使える部品は [サーバーのリファレンス](server-reference.md)。

## 4. クライアントから呼ぶ

生成したラッパーを、`ApiCore` と `CallInvoker` を渡して作る。雛形ではテストの `ApiServerTestBase` で作っている。samples では `SampleLifetimeScope` で VContainer に登録している。

```csharp
builder.Register(resolver => new GachaApi(resolver.Resolve<ApiCore>(), callInvoker), Lifetime.Singleton);
```

呼ぶ側は、結果の `IsSuccess` を見る。例外は投げない。

```csharp
var result = await gachaApi.DrawAsync(new DrawRequest { Free = false });
if (!result.IsSuccess)
{
    return; // エラーダイアログ・画面遷移は ApiCore が済ませている
}
var item = result.Value.Item;
```

- `player_diff`・`badges`・`notifications` は、`ApiCore` がアプリの反映処理(`IPlayerDiffApplier` など)を呼んで反映してから結果を返す
- 画面で自分で処理したいエラーは `handledCodes` に渡す。ダイアログを出さずに失敗として返る

```csharp
var result = await gachaApi.DrawAsync(request, new[] { ErrorCode.InsufficientGems });
if (!result.IsSuccess && result.Error.Code == (int)ErrorCode.InsufficientGems)
{
    ShowShop();
}
```

`ApiCore` が何をするかは [クライアントのリファレンス](client-reference.md)。

## 5. テストを書く

- サーバー: `sandbox/server/server/tests/` に、起動したサーバーに gRPC でつなぐ結合テストを書く(MySQL を使う)
- クライアント: `FakeCallInvoker` に RPC の処理を登録すれば、サーバー無しでラッパーと `ApiCore` を通せる。生成した `GachaServiceMethods.Draw` などのパスの定数を使う([サーバーが無いときの差し替えとテスト](../client/fake-testing.md))

## チェックリスト

- [ ] `.proto` のコメントに、何をするか・どのエラーをいつ返すかを書いた
- [ ] 冪等性(`idempotency_level`)を決めた
- [ ] 新しいエラーコードに既定の action を付けた
- [ ] `make sandbox-proto-gen` で生成したコードをコミットした
- [ ] サーバーのサービスを登録し、`AuthLayer`・`DateChangeLayer` を付けた
- [ ] クライアントのラッパーを登録した
