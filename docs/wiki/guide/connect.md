---
title: "クライアントとサーバーをつなぐ"
status: current
area: guide
related: [guide/getting-started, guide/add-api, guide/roles, guide/debugging, guide/client-reference, guide/server-reference, server/request-flow, server/status, client/api-core]
proto: []
code: [sandbox/server/server/src/lib.rs, sandbox/server/server/src/main.rs, sandbox/server/server/src/player_lookup.rs, sandbox/server/config/server_status.toml, sandbox/UnityProject/Assets/Scripts/Sample/SampleLifetimeScope.cs, Unity/com.chinpangx.voltech.grpc/Runtime/Unity/ApiChannel.cs]
updated: 2026-10-11
---

# クライアントとサーバーをつなぐ

[始め方](getting-started.md) で基盤を参照したあと、アプリのサーバーとクライアントを組み立てて、つながるのを確かめるまでの手順。ここまで済めば、あとは [API を足す](add-api.md) を繰り返す。

実例は sandbox にある。サーバーは `sandbox/server/server/src/lib.rs`、クライアントは `sandbox/UnityProject/Assets/Scripts/Sample/SampleLifetimeScope.cs`(VContainer で組み立てる)。

## 1. サーバーを組み立てる

用意するもの:

- **MySQL**。基盤のテーブル(デバイス・トークン・直前のレスポンス)は、基盤のクレートのマイグレーションが作る。アプリのテーブル(`players` など)は、アプリのマイグレーションで作る
- **`PlayerResolver` の実装**。`player_id` からプレイヤーの状態(`NotCreated`・`Active`・`Suspended`)を返す。`players` テーブルを持つのはアプリなので、アプリが実装する([基盤とアプリの役割分担](roles.md))。sandbox の `PlayerLookup`(`sandbox/server/server/src/player_lookup.rs`)が実例
- **サーバーの状態の設定ファイル**(`server_status.toml`)。メンテナンス・最低バージョン・日付が変わる時刻を書く([サーバーの状態](../server/status.md))。sandbox の `sandbox/server/config/server_status.toml` を写して始める

組み立て方(Layer の順番に意味がある。[リクエストの処理の流れ](../server/request-flow.md)):

```rust
// 起動時: 基盤とアプリのマイグレーション
voltech_grpc_auth::migrate(&pool).await?;
voltech_grpc_transaction::migrate(&pool).await?;
sqlx::migrate!("../migrations").run(&pool).await?;

// サーバーの状態を読み込み、以後 interval ごとに読み直す(書き換えれば再起動しなくても効く)
let status = ServerStatus::watch(FileStatusSource::new(path), Duration::from_secs(5)).await?;

// 冪等性の扱いは、アプリの .proto の idempotency_level から決める
let transactions = Transactions::new(
    pool.clone(),
    IdempotencyLevels::from_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)?,
);
let auth = AuthLayer::new(pool.clone(), PlayerLookup::new(pool.clone())); // PlayerLookup はアプリの PlayerResolver
let date_change = DateChangeLayer::new(status.clone());

Server::builder()
    .layer(ServerStatusLayer::new(status))   // 強制アップデート・メンテナンス(全サービス。認証より先)
    .layer(RpcMethodLayer)                   // Transactions::run が RPC の名前を読めるように
    .add_service(DeviceServiceImpl::new(pool.clone()).into_server())        // 基盤のデバイス登録。認証なし
    .add_service(auth.layer(date_change.layer(GachaServiceImpl::new(transactions).into_server())))
    // ...
```

- アプリのサービスには、すべて `AuthLayer` と `DateChangeLayer` を付ける。付け忘れると、ハンドラーで認証の情報が取れずに INTERNAL になる
- 開発環境では gRPC リフレクションを有効にしておくと、grpcurl や Postman から `.proto` 無しで呼べる。基盤の `voltech_grpc_auth::FILE_DESCRIPTOR_SET` とアプリの FileDescriptorSet を登録する([通信を見る・調べる](debugging.md))

使える型の一覧は [サーバーのリファレンス](server-reference.md)。

## 2. クライアントを組み立てる

必要なものを作って `ApiCore` に渡し、ラッパーに `ApiCore` と `CallInvoker` を渡す。

```csharp
var options = new ApiCoreOptions(UnityAppVersion.Create(appVersion));
var channel = ApiChannel.Create("http://localhost:50051");     // チャンネルは1つを使い回す
var callInvoker = ApiChannel.CreateCallInvoker(channel);        // エディタと開発ビルドでは gRPC Monitor に記録される
var clock = new ServerClock();

var core = new ApiCore(
    options,
    new DeviceTokenProvider(callInvoker, new InMemoryAccountStore(), options), // ITokenProvider
    new ErrorPolicyProvider(AppErrors.All),                                    // IErrorPolicyProvider(アプリの errors.toml から生成した一覧)
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
- デバイス登録とトークンの発行は、最初の通信で `ApiCore` が自動で行う。アプリが呼ぶことはない([認証とアカウント情報](../client/auth-account.md))

アプリが実装する interface:

| interface | 役割 | sandbox の実装 |
|---|---|---|
| `IPlayerDiffApplier` | レスポンスの `player_diff` を、アプリのデータに反映する | `PlayerDiffApplier` |
| `IBadgeApplier` | レスポンスの `badges` で、バッジの状態を更新する | `BadgeApplier` |
| `INotificationApplier` | レスポンスの `notifications` を待ち行列に積む(出すのは画面) | `NotificationApplier` |
| `IErrorDialogView` | エラーダイアログの見た目 | `ErrorDialog` |
| `IErrorNavigator` | エラーの後の画面遷移(タイトル・ホーム・ストア) | `ScreenNavigator` |

- 反映処理(Applier)では、データの更新だけをする。ダイアログや画面遷移は書かない。反映処理で例外が起きたときの扱いは [共通レスポンス項目の反映](../client/response-items.md)
- 生成コードは、共通レスポンス項目を持つレスポンスに `IHasPlayerDiff`・`IHasBadges`・`IHasNotifications` を付ける。反映処理は、この型で判定して読む

使える型の一覧は [クライアントのリファレンス](client-reference.md)。

## 3. つながったか確かめる

1. サーバーを起動する
2. Unity で Play し、最初の API を呼ぶ。`ApiCore` が先にデバイス登録とトークンの発行を行う
3. `Window > VolTech gRPC > gRPC Monitor` を開き、`/voltech.v1.DeviceService/Register`・`IssueToken` と、呼んだ RPC が成功しているかを見る([通信を見る・調べる](debugging.md))

失敗したときに見るところ:

| 症状 | 見るところ |
|---|---|
| どの通信も `Unavailable` になる | `ApiChannel.Create` のアドレスと、サーバーが待ち受けているアドレス。実機からは `localhost` ではつながらない |
| アプリの RPC だけ `INTERNAL` になる | サービスに `AuthLayer`・`DateChangeLayer` を付けたか |
| すべてメンテナンス・強制アップデートになる | `server_status.toml` の `[maintenance]`・`[min_version]` |
