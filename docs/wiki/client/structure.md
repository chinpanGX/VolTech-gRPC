---
title: "構成"
status: confirmed
area: client
related: [repo/current-plan, client/auth-account, client/connection, ops/testing]
proto: []
code: [Unity/com.chinpangx.voltech.grpc/package.json, Unity/com.chinpangx.voltech.grpc/Tests/EditMode/ProtobufVersionTests.cs]
updated: 2026-10-10
---

# 構成

## 依存パッケージ

| パッケージ | 用途 |
|---|---|
| Grpc.Net.Client(と、依存の Grpc.Core.Api) | gRPCクライアント。`RpcException`・`Metadata` などの型 |
| Google.Protobuf | 生成されたメッセージ型のランタイム |
| Google.Api.CommonProtos | エラー詳細の型(`google.rpc.Status` / `google.rpc.ErrorInfo`) |
| UniTask | 非同期処理。基盤のAPI(ラッパー、`ApiCore`、各interface)は UniTask を返す |
| YetAnotherHttpHandler | UnityでHTTP/2通信を行うためのハンドラー(Unity用の層だけが使う) |

- YetAnotherHttpHandler は OpenUPM、それ以外は Unity NuGet レジストリ(`org.nuget.*`)から入れる
- **Google.Protobuf の版は、Google.Api.CommonProtos が依存する版にそろえる**(いまは CommonProtos 2.17.0 に合わせて 3.31.1)。Unity のエディタ(6000.6 以降)は別の版の Google.Protobuf を同梱している。版がずれると、エディタの中で CommonProtos がエディタの版に結び付き、`ErrorInfo` を読むところで `MissingMethodException` になる(業務エラーがすべて不明なエラーになる)。ビルドしたアプリでは起きない。ずれたら `ProtobufVersionTests` が失敗する
- エラー詳細を読む `Grpc.StatusProto` は Unity NuGet レジストリに無いので、トレーラーを読んでパースする処理を自前で書く

## Unityに依存しない本体(Core)と、Unity用の層

> 下の置き場所(`voltech-grpc/unity/`)は、分ける前に考えていた形。実際は `Unity/com.chinpangx.voltech.grpc/` にあり、本体は `Runtime/Client/`(`VolTech.Client`)、認証は `Runtime/Auth/`(`VolTech.Client.Auth`)、Unity 用の層は `Runtime/Unity/`(`VolTech.Client.Unity`)にある([コードの地図](../dev/code-map.md))。

```
voltech-grpc/unity/
├── Core/     本体。ApiCore、エラー処理、共通レスポンス項目の反映、
│              FakeCallInvoker、汎用のテスト、
│              HttpMessageHandler・時計などを受け取る interface
├── Auth/     認証
├── Analyzers/ Roslyn アナライザー
└── Unity/    薄い層。YetAnotherHttpHandler の設定、前面に戻ったときのサーバー時刻のリセット、
               保存先のパス(Application.persistentDataPath)とバンドルID(Application.identifier)を Auth に渡す処理、
               VContainer の登録
```

- 本体が依存するのは UniTask、Grpc.Net.Client、Google.Protobuf、Google.Api.CommonProtos だけ
- **asmdef の参照は GUID で書く**(`"GUID:..."`)。名前で書くと、アセンブリの名前を変えたときに参照が切れる。Unity のインスペクターで参照を足したときの形とも同じになる
- ビルドの種類で入れるかどうかを分けたいもの(開発用のツールなど)は、`#if` で囲まずにアセンブリに分け、asmdef の Define Constraints で決める([通信の Viewer](request-viewer.md))
- 本体は Unity の外(csproj)ではビルドしない。Unity 以外で使う予定が無いので、asmdef だけで持つ。本体のテストも Unity の EditMode テストで回す([テスト](../ops/testing.md))。Unity のライセンスが要るので、v1 では CI では回らない
- 認証(デバイス登録・トークンの発行・アカウント情報の保存)は Auth にある([認証とアカウント情報](auth-account.md))。Auth も、下の本体の決まりを守る
- 生成したラッパーと C# のメッセージ型は、アプリの Unity プロジェクトの中に出力する(samples は `Assets/Scripts/Api/Generated/`、このリポジトリの sandbox は `sandbox/UnityProject/Assets/Generated/`)。基盤の `.proto`(`voltech.v1`)の C# だけはパッケージの `Runtime/Generated/` にある
- **本体の決まり**
  1. UnityEngine を使わない。本体の asmdef で「No Engine References」を有効にし、使った時点でコンパイルエラーにする
  2. `Task` を UniTask にするときは、`AsUniTask()` を使わずにそのまま `await` する。Unity 6 の UniTask は Unity の `Awaitable` 用の `AsUniTask()` も持っていて、どれを使うかを決めるのに UnityEngine の型が要り、No Engine References の asmdef ではコンパイルエラーになる。それ以外の UniTask の機能(`UniTaskCompletionSource`・`WhenAll`・`Delay` など)は使える(EditMode テストで確かめた)
  3. タイムアウトは gRPC の deadline で扱う([接続設定](connection.md))
