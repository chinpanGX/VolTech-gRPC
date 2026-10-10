---
title: "ラッパー"
status: confirmed
area: client
related: [api/codegen, client/connection]
proto: []
code: [sandbox/UnityProject/Assets/Generated/Wrappers]
updated: 2026-10-10
---

# ラッパー

```
Serviceクラス(アプリ)
  │  gachaApi.DrawAsync(request) → ApiResult<DrawResponse>
  ▼
ラッパー(自作のprotocプラグインで生成。gRPCサービスごとに1クラス)
  │  RPCを呼ぶ処理を static ラムダで渡す
  ▼
ApiCore(基盤)  共通処理
  │
  ▼
生成されたgRPCクライアント(grpc_csharp_plugin) → GrpcChannel + YetAnotherHttpHandler
```

```csharp
// 自動生成されるラッパー(protoc-gen-voltech-grpc。実物は Assets/Scripts/Api/Generated/Wrappers/App/V1/GachaApi.cs)
public sealed class GachaApi
{
    private static readonly RpcSettings drawSettings = new(GachaServiceMethods.Draw, isIdempotencyTarget: true, requiresAuth: true);

    public UniTask<ApiResult<DrawResponse>> DrawAsync(
        DrawRequest request, IReadOnlyList<ErrorCode> handledCodes = null)
    {
        return core.InvokeAsync(drawSettings, client, request, handledCodes?.Select(code => (int)code),
            static (c, r, options) => c.DrawAsync(r, options));   // options: ヘッダーとdeadline
    }
}

// 使う側(アプリのServiceクラス)
var result = await gachaApi.DrawAsync(new DrawRequest { BannerId = bannerId });
if (!result.IsSuccess) return;   // 失敗時の共通の処理(ダイアログ等)は済んでいる
// result.Value を使う
```

- 呼び出し側(アプリ)は、生成されたgRPCクライアント、`Metadata`、`RpcException` に触れない
- **gRPCサービスごとに1クラス、RPCごとに1メソッド**。クラス名は gRPCサービス名の末尾の `Service` を `Api` に置き換える(例: `GachaService` → `GachaApi`。[コード生成](../api/codegen.md))
- メソッド名は、生成クライアントと同じ **`{RPC名}Async`**。`.proto` から呼び出し箇所までを名前で追える
- ラッパーは、`ApiCore` に「RPCを呼ぶ処理」を **`static` ラムダ**で渡すだけ。キャプチャしないラムダは delegate が使い回されるので、呼び出しごとのアロケーションは起きない。`static` を付けると、うっかりキャプチャしたときにコンパイルエラーになる
- RPCごとの設定(冪等性の対象か)と、サービスごとの設定(認証が要るか)は、プラグインが `.proto` から読んで埋め込む([コード生成](../api/codegen.md))
- **一度呼んだ通信はキャンセルできない**。ラッパーは `CancellationToken` を受け取らず、`ApiCore` は共通レスポンス項目の反映まで必ず最後まで行う
  - 送信後に打ち切ると、サーバーでは処理が終わっているのにクライアントには反映されない(例: ガチャを引いた直後に画面を閉じると、ジェムは減っているのにクライアントのデータは変わらない)
  - 呼び出した画面が閉じられたときは、通信は止めず、呼び出し側が待つのをやめるだけにする(UniTask の `AttachExternalCancellation`)
  - 応答が返ってこない場合は、deadline([接続設定](connection.md))で扱う
