---
title: "ApiCore の共通処理"
status: confirmed
area: client
related: [api/metadata, client/error-handling, client/auth-account, client/response-items]
proto: []
code: [Unity/com.chinpangx.voltech.grpc/Runtime/Client/ApiCore.cs]
updated: 2026-10-10
---

# ApiCore の共通処理

クライアントの共通処理は、すべて `ApiCore` に置く。v1では gRPC の Interceptor は使わない(生成したラッパーを必ず通るので、処理の入口を1つにできる)。ログや計測を gRPC の層で取りたくなったら、そのときに足す(エディタの [通信の Viewer](request-viewer.md) は、記録するだけの Interceptor を使う)。

| # | 処理 | 補足 |
|---|---|---|
| 1 | 順番待ち | 冪等性の対象(`IDEMPOTENCY_UNKNOWN`)のRPCだけ。成功するか、ユーザーが諦めるまで、次を送らない。応答が届かないまま次を送ると、サーバーの記録が上書きされ、再送で二重実行になるため |
| 2 | RequestIDの採番 | 冪等性の対象のRPCだけ。再送のループの外側で採番し、再送では同じIDを使う |
| 3 | トークンの取得 | Auth を呼ぶ。デバイス登録・発行・期限の確認はパッケージが行う([認証とアカウント情報](auth-account.md)) |
| 4 | ヘッダーの付与 | [メタデータ](../api/metadata.md) |
| 5 | エラーの変換 | [エラー処理](error-handling.md) |
| 6 | `UNAUTHENTICATED` のときの再認証(Auth にトークンを発行し直させる)と、1回だけの再送 | [認証とアカウント情報](auth-account.md) |
| 7 | ログ | |
| 8 | エラー時にエラーハンドラーを待ち、再送するか諦めるかを受け取る | [エラー処理](error-handling.md) |
| 9 | 通信中の入力ブロックと、通信中の表示 | アプリ側のフックを呼ぶ。表示するかは呼び出しごとに指定でき、既定はオン |
| 10 | タイトル・ホームに戻るときの順番待ちの破棄 | 待っているリクエストは送らずに、失敗として終わらせる |
| 11 | サーバー時刻の同期 | 成功したレスポンスのヘッダーのサーバー時刻と、そこからの経過時間(`Stopwatch`)から今のサーバー時刻を計算し、`ApiCore.ServerClock`(`IServerClock`)でアプリに提供する。端末の時計は使わないので、端末の時刻を変えられても影響を受けない。アプリが前面に戻ると、次に API が成功するまで「受け取る前」に戻る(下記) |
| 12 | 日付の保持 | セッションの最初に受け取ったゲーム内の日付を覚え、以降のリクエストに付ける([メタデータ](../api/metadata.md))。受け取るたびには更新しない(更新すると日付の変化に気づけない)。`ResetSession`(タイトルに戻るときも呼ばれる)で消える |
| 13 | 共通レスポンス項目の反映 | [共通レスポンス項目の反映](response-items.md) |
| 14 | 反映処理などで例外が起きたときの後始末 | [共通レスポンス項目の反映](response-items.md) |

- チャネル(`GrpcChannel` + YetAnotherHttpHandler)は1つを使い回す

## 実装で決めたこと

- アプリのプラットフォームとバージョンは `ApiCoreOptions` の `AppVersion` で渡す。`ApiCore` と `DeviceTokenProvider` の両方が付ける(タイトル画面で最初に呼ぶトークン発行で、強制アップデートが分かるように)。Unity では `UnityAppVersion.Create(version)` で作る。エディタでは OS によらず `editor` になる([サーバーの状態](../server/status.md))。バージョンは Unity から実行時に読めないので、アプリが渡す
- サーバー時刻の経過時間は、Unity の `realtimeSinceStartup` ではなく `Stopwatch` で測る。`realtimeSinceStartup` はメインスレッドからしか読めず、通信の後の処理がメインスレッドで動くとは限らないため。どちらも端末の時刻の変更には影響されない
- **ただし iOS・Android の `Stopwatch` は、端末のスリープ中に止まる**。スリープの後はその分だけ遅れるので、アプリが前面に戻ったら(`OnApplicationPause(false)`)`ServerClock.Reset()` で「受け取る前」に戻し、次に API が成功したときに合わせ直す。Unity では `ServerClockResetter.Create(clock)` が行う。アプリは `ServerClock` を自分で作り、`ApiCore` と `ServerClockResetter` の両方に同じものを渡す。画面は、時刻を使う前に `IsSynchronized` を確かめる
