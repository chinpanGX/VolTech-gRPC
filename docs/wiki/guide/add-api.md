---
title: "API を足す"
status: current
area: guide
related: [guide/connect, guide/client-reference, guide/server-reference, guide/debugging, api/codegen, api/errors, api/idempotency, api/common-response, server/game-log]
proto: [sandbox/proto/sandbox/v1/gacha.proto, sandbox/proto/sandbox/v1/player.proto]
code: [sandbox/server/api/src, sandbox/server/gacha/src/service, sandbox/server/player/src/service, sandbox/server/server/src/lib.rs, sandbox/server/proto/build.rs, sandbox/errors.toml, buf.gen.sandbox.yaml]
updated: 2026-10-18
---

# API を足す

アプリに新しい API を足すときの手順。パスとコマンドは、[始め方](getting-started.md) のアプリのフォルダ構成(`client/`・`server/`・`proto/`・`errors.toml`)と Makefile の例で書く。例はガチャ(`GachaService/Draw`)で、同じものが sandbox に動く形で入っている(各節の最後に、sandbox での場所を書く)。`.proto` が API の唯一の定義元で、サーバー(Rust)とクライアント(C#)のコードはそこから生成する([コード生成](../api/codegen.md))。

## 1. `.proto` を書く

アプリの `.proto` は `proto/app/v1/` に置く(パッケージは `app.v1`。`buf.gen.yaml` の `package=` と同じにする)。サービスを持つファイルを足したら、サーバーの生成の対象(サーバーの `build.rs` の `protox::compile` に渡す一覧)にも足す。

```proto
// proto/app/v1/gacha.proto
syntax = "proto3";

package app.v1;

service GachaService {
  // ジェムを消費してガチャを1回引き、排出されたアイテムを1つ付与する。
  rpc Draw(DrawRequest) returns (DrawResponse);
}
```

- **RPC・メッセージ・フィールドの直前のコメントが、API の説明になる**。生成したラッパーの XML ドキュメントにも入る。フィールドには値の形(長さ・文字・単位)まで書く
- **冪等性**: 何も書かなければ「冪等性の対象」になる。クライアントは RequestID を付けて1つずつ送り、サーバーは同じ RequestID の再送で二重に実行しない。読むだけの RPC は `option idempotency_level = NO_SIDE_EFFECTS;`、何回実行しても結果が同じ RPC は `IDEMPOTENT` を書く([冪等性](../api/idempotency.md))
- **認証**: 基盤の `DeviceService` 以外は、すべて認証が要る(アプリが `skip_auth` を使うことは想定していない。[認証](../api/auth.md))
- **共通レスポンス項目**: プレイヤーのデータを変える RPC は、レスポンスに `PlayerDiff player_diff`・`repeated Badge badges`・`repeated Notification notifications` を、この名前で持つ。クライアントの反映処理が自動で読む([共通レスポンス項目](../api/common-response.md))
- **業務エラー**: 新しいエラーはアプリの `errors.toml`(domain はアプリの名前)に、reason・既定の action・description を足す。RPC のコメントに、どのエラーをいつ返すかを書く([エラー](../api/errors.md))

```toml
[[errors]]
reason = "INSUFFICIENT_GEMS"
action = "stay"
description = "ジェムが足りない。"
```

- **一度使ったフィールド番号・名前は使い回さない**。消したら `reserved` にする。リリースした後に番号や型を変えると、古いアプリと通信できなくなる。`buf breaking` で、互換を壊す変更を見つけられる

sandbox では `sandbox/proto/sandbox/v1/gacha.proto`・`sandbox/errors.toml`。

## 2. コードを生成する

```
make proto-gen        # .proto から(buf lint で書き方も確かめる)
make errors-gen       # errors.toml から(エラーを足したとき)
```

- C#: `client/Assets/Generated/` に、メッセージ型(`Messages/`)・gRPC クライアント(`Grpc/`)・ラッパー(`Wrappers/`。`GachaApi` のように、サービスごとに1クラス)・エラーの定数(`Errors/AppErrors.cs`)ができる。生成したコードはコミットする
- Rust: 生成のコマンドは要らない。サーバーをビルドすると、`build.rs` が `.proto` と `errors.toml` から生成する(生成コードはコミットしない)

sandbox では `make sandbox-proto-gen`・`make sandbox-errors-gen`。

## 3. サーバーのハンドラーを書く

サーバーのハンドラーのクレートに、生成されたサービスの trait を実装する。業務の本体は、ドメインのクレート(ガチャなら `gacha`)の `service/` に書く([レイヤー構成](../server/layers.md))。sandbox では、ハンドラーが `sandbox/server/api/src/gacha.rs`、本体が `sandbox/server/gacha/`。本体は `Transactions::run` の中で動かす。トランザクションと、冪等性の対象の RPC の二重実行の防止を、基盤がまとめて行う。

```rust
async fn draw(&self, request: Request<DrawRequest>) -> Result<Response<DrawResponse>, Status> {
    let player_id = player_id(&request)?; // AuthLayer が入れた認証の情報から取る
    let response = self
        .transactions
        .run(&request, Some(player_id), move |ctx| {
            Box::pin(async move {
                // ctx.tx() で DB を読み書きし、所持データを変えたら ctx.write_game_log でゲームログを書く。
                // 業務エラーは failed_precondition で返す
                gacha::service::draw(ctx, /* ... */).await.map_err(|err| match err {
                    DrawError::InsufficientGems => {
                        failed_precondition(errors::INSUFFICIENT_GEMS, "not enough gems")
                    }
                    DrawError::Internal(err) => err.into(), // INTERNAL。原因はログにだけ出る
                    // ...
                })?;
                Ok(DrawResponse { /* ... */ })
            })
        })
        .await?;
    Ok(Response::new(response))
}
```

業務の本体は、DB のエラーなどの原因を `voltech_grpc_error::Internal` で運ぶ。`?` で `Internal` になり、ハンドラーで `Status` に戻すと INTERNAL になって、原因の連なりと発生した場所がログに出る。業務エラーの `Status` を `?` で包んでも、`Status` に戻すときは元の業務エラーのまま返る。

```rust
pub async fn consume_gems(ctx: &mut TxContext, /* ... */) -> Result<Option<i64>, Internal> {
    sqlx::query("UPDATE players SET gems = gems - ? WHERE player_id = ?")
        /* ... */
        .execute(&mut **ctx.tx())
        .await?;                                                       // sqlx のエラーは ? で Internal に
    ctx.write_game_log(GameLog::new("gems", -amount, remaining, reason))?; // ジェムの増減を残す
    /* ... */
}
```

ゲームログは、所持データ(ジェム・アイテムなど)を変えたところで書く。処理が失敗してロールバックしたときと、再送で保存済みのレスポンスを返したときには残らない([ゲームログ](../server/game-log.md))。

作ったサービスは、サーバーを組み立てるところ(sandbox では `sandbox/server/server/src/lib.rs`)で登録する。認証が要るサービスには `AuthLayer` と `DateChangeLayer` を付ける(付け忘れると、ハンドラーで認証の情報が取れずに INTERNAL になる)。

```rust
.add_service(auth.layer(date_change.layer(GachaServiceImpl::new(transactions).into_server())))
```

使える部品は [サーバーのリファレンス](server-reference.md)。

## 4. クライアントから呼ぶ

生成したラッパーを、`ApiCore` と `CallInvoker` を渡して作る。VContainer なら、`LifetimeScope` で登録する(sandbox では `SampleLifetimeScope`)。

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
- 画面を止めずに裏で呼ぶもの(定期的な更新など)は `notifyActivity: false` にする。通信中の表示と入力ブロックの対象にならない

```csharp
var result = await gachaApi.DrawAsync(request, new[] { AppErrors.InsufficientGems });
if (!result.IsSuccess && result.Error.Code == AppErrors.InsufficientGems)
{
    ShowShop();
}
```

`ApiCore` が何をするかは [クライアントのリファレンス](client-reference.md)。

## 5. テストを書く

- サーバー: 起動したサーバーに gRPC でつなぐ結合テストを書く(MySQL を使う。tonic が生成するクライアントで呼ぶと、実際のクライアントと同じ経路を通る。sandbox では `sandbox/server/server/tests/`)
  - `IDEMPOTENT` と宣言した RPC は、2回呼んでも状態が同じになることを確かめる
  - 冪等性の対象の RPC は、同じ RequestID で2回送ると、業務データが1回分しか変わらないことを確かめる
- クライアント: `FakeCallInvoker` に RPC の処理を登録すれば、サーバー無しでラッパーと `ApiCore` を通せる。生成した `GachaServiceMethods.Draw` などのパスの定数を使う([サーバーが無いときの差し替えとテスト](../client/fake-testing.md))
- エラーを足したら、そのエラーの文言(ダイアログに出す文)がアプリの文言データにあるかを確かめる

## チェックリスト

- [ ] `.proto` のコメントに、何をするか・どのエラーをいつ返すかを書いた
- [ ] 冪等性(`idempotency_level`)を決めた
- [ ] 新しいエラーを `errors.toml` に書き、既定の action を付けた
- [ ] `make proto-gen`・`make errors-gen` で生成したコードをコミットした
- [ ] サーバーのサービスを登録し、`AuthLayer`・`DateChangeLayer` を付けた
- [ ] クライアントのラッパーを登録した
