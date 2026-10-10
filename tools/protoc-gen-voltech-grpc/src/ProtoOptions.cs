using Google.Protobuf;
using Google.Protobuf.Reflection;

namespace ProtocGenVoltechGrpc;

/// <summary>
/// ラッパーに埋め込む、.proto のオプションを読む。
/// </summary>
internal static class ProtoOptions
{
    // voltech/v1/auth_options.proto の skip_auth の番号。プラグインと認証のクレート(voltech-grpc-auth)は同じタグで出すので、番号で読む。
    // アプリの .proto は voltech/v1/auth_options.proto を import しないので、リクエストから番号を引くことはできない
    private const int SkipAuthFieldNumber = 50002;

    /// <summary>
    /// 冪等性の対象か。idempotency_level を書いていない RPC が対象(RequestID を付け、1つずつ送る)。
    /// </summary>
    public static bool IsIdempotencyTarget(MethodDescriptorProto method)
    {
        var level = method.Options?.IdempotencyLevel ?? MethodOptions.Types.IdempotencyLevel.IdempotencyUnknown;
        return level == MethodOptions.Types.IdempotencyLevel.IdempotencyUnknown;
    }

    /// <summary>
    /// 認証なしで呼べるサービスか(サービスのオプション skip_auth)。
    /// </summary>
    public static bool SkipAuth(ServiceDescriptorProto service)
    {
        if (service.Options == null)
        {
            return false;
        }

        // 拡張を登録していないので、skip_auth は未知のフィールドとして残っている。書き出して番号で探す
        var input = new CodedInputStream(service.Options.ToByteArray());
        uint tag;
        while ((tag = input.ReadTag()) != 0)
        {
            if (WireFormat.GetTagFieldNumber(tag) == SkipAuthFieldNumber)
            {
                return input.ReadBool();
            }

            input.SkipLastField();
        }

        return false;
    }
}
