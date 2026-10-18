---
title: "クライアントのリファレンス"
status: current
area: guide
related: [guide/add-api, client/api-core, client/error-handling, client/response-items, client/auth-account, client/fake-testing, client/structure]
proto: []
code: [Unity/com.chinpangx.voltech.grpc/Runtime/Client, Unity/com.chinpangx.voltech.grpc/Runtime/Unity, Unity/com.chinpangx.voltech.grpc/Runtime/Client/Errors/ErrorPolicyProvider.cs, sandbox/UnityProject/Assets/Scripts/Sample/SampleLifetimeScope.cs]
updated: 2026-10-18
---

# クライアントのリファレンス

Unity のクライアントで使う、基盤の型の一覧。各型の細かい説明は、コードの XML ドキュメント(`///`)が正。ここでは、どの型を何に使うかと、組み込み方を書く。

| アセンブリ / フォルダ | 名前空間 | 中身 |
|---|---|---|
| `VolTech.Client`(`Runtime/Client/`) | `VolTech.Client`・`VolTech.Client.Auth` | 基盤の本体と、エラーの既定の action の表(`ErrorPolicyProvider`)・予約エラー(`ReservedErrors`)。認証(`DeviceTokenProvider`)とアカウントの保存先は `Auth/`(名前空間 `VolTech.Client.Auth`)。Unity に依存しない(No Engine References) |
| `VolTech.Client.Unity`(`Runtime/Unity/`) | `VolTech.Client.Unity` | Unity 用の層(`ApiChannel`・`UnityApiLogger` など) |
| `VolTech.V1`(`Runtime/Generated/`) | `VolTech.V1` | 基盤の `.proto`(デバイス登録・カスタムオプション)から生成した C# |
| アプリの生成コード(アプリの Unity プロジェクトの中) | `App.V1` など(アプリが決める) | アプリの `.proto` から生成したメッセージ型・gRPC クライアント・ラッパー |

フォルダはパッケージ(`Unity/com.chinpangx.voltech.grpc/`)の中のもの。アプリの asmdef が参照するのは `VolTech.Client`・`VolTech.Client.Unity`・`VolTech.V1` の3つ(`VolTech.V1` は、基盤の `.proto` の型を使うアプリの生成コードが参照する)。アプリから使わないものは `internal` にしてあるので、ここに無い型はアプリからは見えない。組み立て方とアプリが実装する interface は [クライアントとサーバーをつなぐ](connect.md)。

## 呼ぶ

| 型 | 使い方 |
|---|---|
| ラッパー(`GachaApi` など。生成) | サービスごとに1クラス。RPC ごとに `XxxAsync(request, handledCodes, notifyActivity)` を持ち、`ApiResult<TResponse>` を返す。`handledCodes` に渡したエラーコードは、ダイアログを出さずに失敗として返る。`notifyActivity: false` の呼び出しは、通信中として知らせない |
| `XxxServiceMethods`(生成) | RPC のパスの定数(`"/app.v1.GachaService/Draw"`)。`FakeCallInvoker` への登録に使う |
| `ApiResult<T>` | 結果。`IsSuccess`・`Value`(成功のとき)・`Error`(失敗のとき)。失敗のときに `Value` を読むと例外になる |
| `ApiError` | 失敗の内容。`Code`(エラーコード)・`Action`(取る行動)・`StatusCode`(gRPC のステータス)・`Metadata`(表示用の値。例: メンテナンスの `end_at`)・`IsDiscarded`(タイトル・ホームに戻ったので送らずに捨てた) |
| `ApiCore` | 共通処理。`ApiCore.CreateBuilder(...)` で作った `ApiCoreBuilder` に任意の部品を渡し、`Build()` で作る。ラッパーが呼ぶので、アプリが直接呼ぶのは `ResetSession()`(アカウントを入れ替えた後)と `ServerClock` くらい。RPC を呼ぶ処理は、生成したラッパーが基底クラス `ApiServiceBase` を通して使う(アプリは継承しない) |

`ApiCore` がまとめて行うこと: トークンとヘッダーの付与、冪等性の対象の RPC の順番待ちと RequestID、UNAUTHENTICATED のときのトークンの発行し直しと再送、エラーの変換とエラーハンドラーの呼び出し、サーバー時刻とゲーム内の日付の保持、共通レスポンス項目の反映、通信中の開始・終了の通知(`IApiActivityHandler`)。詳しくは [ApiCore の共通処理](../client/api-core.md)。

## 基盤が用意している実装

| 型 | 役割 |
|---|---|
| `DeviceTokenProvider`(`ITokenProvider`) | デバイス登録・トークンの発行・期限の管理を自動で行う。アカウント情報は `IAccountStore` に保存する。`GetAccountStatusAsync()` で、保存したアカウント情報の状態(`AccountStatus`: `NotRegistered`・`Registered`・`Corrupted`)を返す。タイトル画面でサインアップとサインインのどちらに進むかを決めるのに使う([認証とアカウント情報](../client/auth-account.md)) |
| `IAccountStore` / `EncryptedFileAccountStore` | アカウント情報(`AccountData`: `DeviceId`・`SecretKey`・`PlayerId`)の保存先。`EncryptedFileAccountStore` は暗号化したファイルに保存する(Unity では `UnityAccountStore.Create(serverAddress)` で作る)。ファイルがあるのに読めないと `CorruptedAccountDataException` を投げる。新しいプレイヤーで始め直すときは、`ClearAsync()` のあとに `ApiCore.ResetSession()` を呼ぶ(開発中も同じ。sandbox のタイトル画面のデータ削除が見本) |
| `DefaultCommonErrorHandler`(`ICommonErrorHandler`) | action ごとに決まったボタンのダイアログを出し、押されたボタンから動き(`ErrorDecision`)を決める([エラー処理](../client/error-handling.md)) |
| `ErrorPolicyProvider`(`IErrorPolicyProvider`) | エラーコードから既定の action を引く。アプリの `errors.toml` から生成した一覧(例: `AppErrors.All`)を渡して作る。予約エラーは自動で入る([エラー](../api/errors.md)) |
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

## テストで使う

| 型 | 使い方 |
|---|---|
| `FakeCallInvoker` | 通信する代わりに、RPC ごとに登録した処理を呼ぶ。生成したクライアント・ラッパーに渡すと、`ApiCore` とエラー処理は本物と同じに動く。`Calls` で呼ばれた RPC とヘッダーを確かめられる |
| `FakeRpcErrors` | サーバーが返すエラーを作る。`FailedPrecondition(code, message, metadata)` は業務エラー(`ErrorInfo` 付き)、`Status(code)` はそれ以外 |
| `InMemoryAccountStore` | メモリに持つだけの `IAccountStore`。テスト用で、アプリでは使わない(起動するたびに別の端末として登録され、前のプレイヤーに戻れなくなる) |

```csharp
var invoker = new FakeCallInvoker()
    .Register<DrawRequest, DrawResponse>(GachaServiceMethods.Draw, (request, headers) =>
        UniTask.FromResult(new DrawResponse { Item = new PlayerItem { ItemId = 3, Count = 1 } }));
var gachaApi = new GachaApi(core, invoker);
```

詳しくは [サーバーが無いときの差し替えとテスト](../client/fake-testing.md)。

## 開発用のツール

`VolTech.Client.Tracing`(エディタと開発ビルドだけ)と `VolTech.Client.Editor`(エディタだけ)は、gRPC Monitor のためのアセンブリ。アプリは何もしなくても、`ApiChannel.CreateCallInvoker` で作った通信が記録される([通信を見る・調べる](debugging.md))。
