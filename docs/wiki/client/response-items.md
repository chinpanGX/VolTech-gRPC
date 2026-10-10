---
title: "共通レスポンス項目の反映"
status: confirmed
area: client
sections: ["5.6"]
related: [api/codegen, api/common-response, client/fake-testing]
proto: []
code: [Unity/com.chinpangx.voltech.grpc/Runtime/Client/Hooks]
updated: 2026-10-10
---

# 共通レスポンス項目の反映

`ApiCore` は、成功したら呼び出し側に返す前に、共通レスポンス項目([3.9](../api/common-response.md))をアプリ側の反映処理に渡す。順番は playerDiff → バッジ → 通知。

```csharp
// PlayerDiff などはアプリ側の型なので、基盤はレスポンスごと渡す
public interface IPlayerDiffApplier   { UniTask ApplyAsync(IMessage response); }   // player_diff を Repository に反映する
public interface IBadgeApplier        { UniTask ApplyAsync(IMessage response); }   // badges でバッジの状態を更新する(含まれない種類は触らない)
public interface INotificationApplier { UniTask ApplyAsync(IMessage response); }   // notifications を待ち行列に積む
```

- **反映処理は、`.proto` でそのフィールドを使っているときだけ必須にする**。使っているのに登録を忘れたら、起動時にエラーにする(プラグインが生成した一覧で確かめる。[3.4](../api/codegen.md))
- **順番待ちの中で行う**。次のAPIが送られる時点で、必ず反映が終わっている
- **反映処理でやるのはデータの更新だけ**にし、ダイアログや画面遷移は書かない。通知を出すタイミングはアプリの画面が決める(反映処理の中で出すと、ガチャの演出の途中に割り込む)
- `IPlayerDiffApplier` の中では、リソースごとの反映を並列(`UniTask.WhenAll`)に呼ぶ。反映の順序に依存する作りにはしない
- **APIごとの処理は、呼び出し側が `await` で結果を受け取った後に書く**(例: サインインの全件を置き換える)。全APIに共通の処理が必要になったら、共通レスポンス項目として足す
- 変化前の値が必要な演出(レベルアップ前のレベルなど)は、呼び出す前に値を取っておく(返った時点で反映が終わっているため)
- サーバーで計算したバッジと、クライアントで判定したバッジ([3.9](../api/common-response.md))は、アプリ側(`BadgeService` など)で種類ごとにまとめて画面に見せる

当面の実装:

- 反映処理は `IPlayerDiffApplier`・`IBadgeApplier`・`INotificationApplier` で、`ApiCore` のコンストラクタで受け取る(どれも省略できる)。player_diff → バッジ → 通知の順に呼び、途中で例外が起きたら残りは呼ばない
- 「使っているのに登録を忘れたら起動時にエラー」は、まだ入れていない(プラグインが一覧を生成するときに足す)
- アプリの `BadgeApplier` は、`IHasBadges` のレスポンスの `badges` を `BadgeStore` に種類ごとに上書きする。知らない種類(古いアプリにとって、サーバーが後から足した種類)は無視する
- アプリの `NotificationApplier` は、`IHasNotifications` のレスポンスの `notifications` を `NotificationQueue` に届いた順に積む。`oneof` が空のもの(知らない種類)は積まない。通知を取り出して出すのは画面
- `ApiCore` は、通信の `try` の外で反映処理を呼ぶ。通信のエラーと同じ扱いにすると、エラーハンドラーが再試行を選んだときに送り直してしまうため
- アプリの `PlayerDiffApplier` は、レスポンスが `IHasPlayerDiff`(`player_diff` を持つレスポンスに protoc-gen-voltech-grpc が付ける。[コード生成](../api/codegen.md))なら `PlayerDiff` を読み、`PlayerRepository` に反映する。フィールド名の文字列やリフレクションは使わない
- `PlayerRepository` はレスポンスの型を知らず、データの操作(`SetGems`・`UpsertItems`・`RemoveItems`・`ClearItems`)だけを持つ。差分の値は変更後の値なので、反映は上書き(Upsert)になり、同じ差分を2回反映しても結果は変わらない。無くなったものは Upsert では表せないので、`removed` で消す
  - player_diff: `PlayerDiffApplier` が、差分に入っているものだけを `SetGems` → `UpsertItems` → `RemoveItems` で反映する
  - サインインの全件: 呼び出し側が `SetGems` → `ClearItems` → `UpsertItems` で置き換える。全件には `removed` が無いので、先に消さないと、サーバーで無くなったアイテムが残る

## 反映処理で例外が起きたとき

反映処理の例外は、アプリ側の実装のバグ。サーバー側の処理は成功しているので、再送はしない。

- 順番待ちと通信中の入力ブロックを**必ず解除する**(`finally`)。解除しないと、以降のAPIがすべて待たされ、フリーズしたように見える
- 例外の内容(どの反映処理で、どのレスポンスのときか)をスタックトレース付きでログに出し、`CLIENT_ERROR` として共通エラーハンドラーに渡す。タイトルに戻ってサインインで全件を取り直し、サーバーとの食い違いを解消する
- 呼び出し側には失敗を返し、続きの処理は動かさない
- 開発ビルドとリリースビルドで動きは変えない。開発中に必ず気づけるように、全APIの反映処理をテストで通す([5.8](fake-testing.md))

## 採らなかった案

| 項目 | 採らなかった案 | 理由 |
|---|---|---|
| 共通レスポンス項目の反映 | 全APIで汎用に使える成功後のフック(`IApiResponseHook`) | APIごとの処理は呼び出し側に書けば足りる。共通の処理は共通レスポンス項目として足す |
| 共通レスポンス項目の反映 | アプリのServiceクラスがそれぞれ反映する(実務・Atlasの方式) | 呼び忘れがたびたび起きた |
| Repository の操作 | API のレスポンスの型で受け取る(`ReplaceAll(SignInResponse)`・`Apply(PlayerDiff)`) | Repository が API を知ることになり、全件を返す API が増えるたびにメソッドが増える。何をするのかも名前から読めない |
| アイテムの削除 | `removed` を無くし、個数 0 の Upsert で消す | 個数を持たないもの(キャラクター・装備など)の売却を表せない |
