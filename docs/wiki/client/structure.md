---
title: "構成"
status: confirmed
area: client
related: [repo/operations, client/auth-account, client/connection, appendix/testing]
proto: []
code: [Unity/com.chinpangx.voltech.grpc/package.json, Unity/com.chinpangx.voltech.grpc/Runtime/Client/Errors/RpcErrorDetails.cs]
updated: 2026-10-11
---

# 構成

## 依存パッケージ

| パッケージ | 用途 |
|---|---|
| Grpc.Net.Client(と、依存の Grpc.Core.Api) | gRPCクライアント。`RpcException`・`Metadata` などの型 |
| Google.Protobuf | 生成されたメッセージ型のランタイム |
| UniTask | 非同期処理。基盤のAPI(ラッパー、`ApiCore`、各interface)は UniTask を返す |
| YetAnotherHttpHandler | UnityでHTTP/2通信を行うためのハンドラー(Unity用の層だけが使う) |

- YetAnotherHttpHandler は OpenUPM、それ以外は Unity NuGet レジストリ(`org.nuget.*`)から入れる
- **Google.Protobuf を参照するビルド済みの DLL(Google.Api.CommonProtos など)には依存しない**。Unity のエディタ(6000.6 以降)は別の版の Google.Protobuf を同梱していて、DLL が参照する版がパッケージに無いと、エディタの中でその DLL がエディタの版に結び付き、型が食い違って `MissingMethodException` になる(ビルドしたアプリでは起きない)。`make deps-check` が、lock の中にそういうパッケージが無いことを確かめる
- エラー詳細(`google.rpc.Status` / `google.rpc.ErrorInfo`)は、`RpcErrorDetails` が Google.Protobuf の `CodedInputStream` で自前で読む(`Grpc.StatusProto` は Unity NuGet レジストリに無く、CommonProtos は上の理由で使わない)。CommonProtos と同じバイト列になることを `RpcErrorDetailsTests` が確かめる

## Unityに依存しない本体(Core)と、Unity用の層

> 下の置き場所(`voltech-grpc/unity/`)は、分ける前に考えていた形。実際は `Unity/com.chinpangx.voltech.grpc/` にあり、本体は `Runtime/Client/`(`VolTech.Client`)、認証は `Runtime/Auth/`(`VolTech.Client.Auth`)、Unity 用の層は `Runtime/Unity/`(`VolTech.Client.Unity`)にある([コードの地図](../appendix/code-map.md))。

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

- 本体が依存するのは UniTask、Grpc.Net.Client、Google.Protobuf だけ
- **asmdef の参照は GUID で書く**(`"GUID:..."`)。名前で書くと、アセンブリの名前を変えたときに参照が切れる。Unity のインスペクターで参照を足したときの形とも同じになる
- ビルドの種類で入れるかどうかを分けたいもの(開発用のツールなど)は、`#if` で囲まずにアセンブリに分け、asmdef の Define Constraints で決める([gRPC Monitor](grpc-monitor.md))
- 本体は Unity の外(csproj)ではビルドしない。Unity 以外で使う予定が無いので、asmdef だけで持つ。本体のテストも Unity の EditMode テストで回す([テスト](../appendix/testing.md))。Unity のライセンスが要るので、v1 では CI では回らない
- 認証(デバイス登録・トークンの発行・アカウント情報の保存)は Auth にある([認証とアカウント情報](auth-account.md))。Auth も、下の本体の決まりを守る
- 生成したラッパーと C# のメッセージ型は、アプリの Unity プロジェクトの中に出力する(sandbox は `sandbox/UnityProject/Assets/Generated/`。`errors.toml` から生成したエラーコードも同じ場所の `Errors/`)。基盤の `.proto`(`voltech.v1`)の C# だけはパッケージの `Runtime/Generated/` にある
- **本体の決まり**
  1. UnityEngine を使わない。本体の asmdef で「No Engine References」を有効にし、使った時点でコンパイルエラーにする
  2. `Task` を UniTask にするときは、`AsUniTask()` を使わずにそのまま `await` する。Unity 6 の UniTask は Unity の `Awaitable` 用の `AsUniTask()` も持っていて、どれを使うかを決めるのに UnityEngine の型が要り、No Engine References の asmdef ではコンパイルエラーになる。それ以外の UniTask の機能(`UniTaskCompletionSource`・`WhenAll`・`Delay` など)は使える(EditMode テストで確かめた)
  3. タイムアウトは gRPC の deadline で扱う([接続設定](connection.md))
