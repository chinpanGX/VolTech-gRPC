---
title: "クライアントのリファレンス"
status: current
area: guide
related: [guide/add-api, client/api-core, client/error-handling, client/response-items, client/auth-account, client/fake-testing, client/structure]
proto: []
code: [Unity/com.chinpangx.voltech.grpc/Runtime/Client, Unity/com.chinpangx.voltech.grpc/Runtime/Auth, Unity/com.chinpangx.voltech.grpc/Runtime/Unity, Unity/com.chinpangx.voltech.grpc/Runtime/Client/Errors/ErrorPolicyProvider.cs, sandbox/UnityProject/Assets/Scripts/Sample/SampleLifetimeScope.cs]
updated: 2026-10-11
---

# クライアントのリファレンス

Unity のクライアントで使う、基盤の型の一覧。各型の細かい説明は、コードの XML ドキュメント(`///`)が正。ここでは、どの型を何に使うかと、組み込み方を書く。

| アセンブリ / フォルダ | 名前空間 | 中身 |
|---|---|---|
| `VolTech.Client`(`Runtime/Client/`) | `VolTech.Client` | 基盤の本体と、エラーの既定の action の表(`ErrorPolicyProvider`)・予約エラー(`ReservedErrors`)。Unity に依存しない(No Engine References) |
| `VolTech.Client.Auth`(`Runtime/Auth/`) | `VolTech.Client.Auth` | 認証(`DeviceTokenProvider`)とアカウントの保存先 |
| `VolTech.Client.Unity`(`Runtime/Unity/`) | `VolTech.Client.Unity` | Unity 用の層(`ApiChannel`・`UnityApiLogger` など) |
| `VolTech.V1`(`Runtime/Generated/`) | `VolTech.V1` | 基盤の `.proto`(デバイス登録・カスタムオプション)から生成した C# |
| アプリの生成コード(アプリの Unity プロジェクトの中) | `App.V1` など(アプリが決める) | アプリの `.proto` から生成したメッセージ型・gRPC クライアント・ラッパー |

フォルダはパッケージ(`Unity/com.chinpangx.voltech.grpc/`)の中のもの。

## 組み込み方

sandbox の `SampleLifetimeScope`(`sandbox/UnityProject/Assets/Scripts/Sample/`。VContainer で組み立てる)と、テストの `ApiServerTestBase` が実例。必要なものを作って `ApiCore` に渡し、ラッパーに `ApiCore` と `CallInvoker` を渡す。

```csharp
var options = new ApiCoreOptions(UnityAppVersion.Create(appVersion));
var channel = ApiChannel.Create("http://localhost:50051");     // チャンネルは1つを使い回す
var callInvoker = ApiChannel.CreateCallInvoker(channel);        // エディタと開発ビルドでは gRPC Monitor に記録される
var clock = new ServerClock();

var core = new ApiCore(
    options,
    new DeviceTokenProvider(callInvoker, new InMemoryAccountStore(), options), // ITokenProvider
    new ErrorPolicyProvider(SandboxErrors.All),                                // IErrorPolicyProvider(アプリの errors.toml から生成した一覧)
    new DefaultCommonErrorHandler(errorDialog),                                // ICommonErrorHandler
    new PlayerDiffApplier(playerRepository),                                   // IPlayerDiffApplier(アプリ)
    new UnityApiLogger(),                                                      // IApiLogger(リリースでは null)
    clock,
    navigator,                                                                 // IErrorNavigator(アプリ)
    new BadgeApplier(badgeStore),                                              // IBadgeApplier(アプリ)
    new NotificationApplier(notificationQueue));                               // INotificationApplier(アプリ)

var gachaApi = new GachaApi(core, callInvoker);
```

- 前面に戻ったときにサーバー時刻を合わせ直すため、`ServerClockResetter.Create(clock)` も作る
- `ApiCore`・ラッパー・`CallInvoker` はアプリで1つずつにする(順番待ちとセッションを `ApiCore` が持つため)

## 呼ぶ

| 型 | 使い方 |
|---|---|
| ラッパー(`GachaApi` など。生成) | サービスごとに1クラス。RPC ごとに `XxxAsync(request, handledCodes)` を持ち、`ApiResult<TResponse>` を返す。`handledCodes` に渡したエラーコードは、ダイアログを出さずに失敗として返る |
| `XxxServiceMethods`(生成) | RPC のパスの定数(`"/app.v1.GachaService/Draw"`)。`FakeCallInvoker` への登録に使う |
| `ApiResult<T>` | 結果。`IsSuccess`・`Value`(成功のとき)・`Error`(失敗のとき)。失敗のときに `Value` を読むと例外になる |
| `ApiError` | 失敗の内容。`Code`・`CodeName`(エラーコード)・`Action`(取る行動)・`StatusCode`(gRPC のステータス)・`Metadata`(表示用の値。例: メンテナンスの `end_at`)・`IsDiscarded`(タイトル・ホームに戻ったので送らずに捨てた) |
| `ApiCore` | 共通処理。ラッパーが呼ぶので、アプリが直接呼ぶのは `ResetSession()`(アカウントを入れ替えた後)と `ServerClock` くらい |

`ApiCore` がまとめて行うこと: トークンとヘッダーの付与、冪等性の対象の RPC の順番待ちと RequestID、UNAUTHENTICATED のときのトークンの発行し直しと再送、エラーの変換とエラーハンドラーの呼び出し、サーバー時刻とゲーム内の日付の保持、共通レスポンス項目の反映。詳しくは [ApiCore の共通処理](../client/api-core.md)。

## アプリが実装する interface

| interface | 役割 | サンプルの実装 |
|---|---|---|
| `IPlayerDiffApplier` | レスポンスの `player_diff` を、アプリのデータに反映する | `PlayerDiffApplier` |
| `IBadgeApplier` | レスポンスの `badges` で、バッジの状態を更新する | `BadgeApplier` |
| `INotificationApplier` | レスポンスの `notifications` を待ち行列に積む(出すのは画面) | `NotificationApplier` |
| `IErrorDialogView` | エラーダイアログの見た目 | `ErrorDialog` |
| `IErrorNavigator` | エラーの後の画面遷移(タイトル・ホーム・ストア) | `ScreenNavigator` |

- 反映処理(Applier)では、データの更新だけをする。ダイアログや画面遷移は書かない。反映処理で例外が起きたときの扱いは [共通レスポンス項目の反映](../client/response-items.md)
- 生成コードは、共通レスポンス項目を持つレスポンスに `IHasPlayerDiff`・`IHasBadges`・`IHasNotifications` を付ける。反映処理は、この型で判定して読む

## 基盤が用意している実装

| 型 | 役割 |
|---|---|
| `DeviceTokenProvider`(`ITokenProvider`) | デバイス登録・トークンの発行・期限の管理を自動で行う。アカウント情報は `IAccountStore` に保存する([認証とアカウント情報](../client/auth-account.md)) |
| `IAccountStore` / `InMemoryAccountStore` | アカウント情報(`AccountData`: `DeviceId`・`SecretKey`・`PlayerId`)の保存先。`InMemoryAccountStore` はメモリだけ(起動するたびに新しい端末になる)。端末に保存する実装は、まだ無い |
| `DefaultCommonErrorHandler`(`ICommonErrorHandler`) | action ごとに決まったボタンのダイアログを出し、押されたボタンから動き(`ErrorDecision`)を決める([エラー処理](../client/error-handling.md)) |
| `ErrorPolicyProvider`(`IErrorPolicyProvider`) | エラーコードから既定の action を引く。アプリの `errors.toml` から生成した一覧(例: `SandboxErrors.All`)を渡して作る。予約エラーは自動で入る([エラー](../api/errors.md)) |
| `ServerClock`(`IServerClock`) | サーバー時刻。成功したレスポンスのヘッダーで合わせる。`IsSynchronized` が false の間は `Now` を読めない |
| `ApiChannel` | `Create(address)` で YetAnotherHttpHandler(HTTP/2)の `GrpcChannel` を作る。`CreateCallInvoker(channel)` で、開発用の Interceptor(通信の記録)を付けた `CallInvoker` を作る |
| `UnityAppVersion` | `Create(version)` で、今のプラットフォームとバージョンの `AppVersion` を作る。サーバーはこれで強制アップデートを判定する |
| `UnityApiLogger`(`IApiLogger`) | 通信のログを `Debug.Log` に出す。開発中だけ渡す |

## 値と定数

| 型 | 中身 |
|---|---|
| `ApiCoreOptions` | `Deadline`(1回の通信全体の制限時間。15 秒)・`AppVersion` |
| `AppVersion` | プラットフォーム(`Ios`・`Android`・`Windows`・`Editor`)と、リリースのたびに増やす整数のバージョン |
| `ErrorAction` | エラーの後に取る行動(`Stay`・`Retry`・`Home`・`Title`・`Update`)。`errors.toml` の action に対応する |
| `ErrorCode` | エラーコード。`ErrorInfo` の domain と reason の組(`ApiError.Code` の型)。`handledCodes` にも渡す |
| `ErrorDecision` | エラーハンドラーが決めた動き(送り直す・留まる・ホーム・タイトル・ストア) |
| `ReservedErrors` | 基盤の予約エラー(domain `voltech`。`server/rust/error/errors.toml` から生成)。`Maintenance`・`NetworkError` など |
| `ApiHeaders` | ヘッダー名(`authorization`・`x-request-id`・`x-app-platform`・`x-app-version`・`x-game-date`・`x-server-time`。[メタデータ](../api/metadata.md)) |
| `RpcErrorDetails` | `GetErrorInfo(RpcException)` で、トレーラーの `grpc-status-details-bin` から `ErrorInfo` を読む |

## テストで使う

| 型 | 使い方 |
|---|---|
| `FakeCallInvoker` | 通信する代わりに、RPC ごとに登録した処理を呼ぶ。生成したクライアント・ラッパーに渡すと、`ApiCore` とエラー処理は本物と同じに動く。`Calls` で呼ばれた RPC とヘッダーを確かめられる |
| `FakeRpcErrors` | サーバーが返すエラーを作る。`FailedPrecondition(codeName, message, metadata)` は業務エラー(`ErrorInfo` 付き)、`Status(code)` はそれ以外 |

```csharp
var invoker = new FakeCallInvoker()
    .Register<DrawRequest, DrawResponse>(GachaServiceMethods.Draw, (request, headers) =>
        UniTask.FromResult(new DrawResponse { Item = new PlayerItem { ItemId = 3, Count = 1 } }));
var gachaApi = new GachaApi(core, invoker);
```

詳しくは [サーバーが無いときの差し替えとテスト](../client/fake-testing.md)。

## 開発用のツール

`VolTech.Client.Tracing`(エディタと開発ビルドだけ)と `VolTech.Client.Editor`(エディタだけ)は、gRPC Monitor のためのアセンブリ。アプリは何もしなくても、`ApiChannel.CreateCallInvoker` で作った通信が記録される([ツール](tools.md))。
