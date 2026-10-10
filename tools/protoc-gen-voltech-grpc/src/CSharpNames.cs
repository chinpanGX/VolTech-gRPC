using System.Text;
using Google.Protobuf.Reflection;

namespace ProtocGenVoltechGrpc;

/// <summary>
/// .proto の名前を、protoc 組み込みの C# 生成と同じ規則で C# の名前にする。
/// </summary>
/// <remarks>
/// 生成したコードは、protoc が生成したメッセージ型・grpc_csharp_plugin が生成したクライアントを参照するので、
/// 名前の付け方をそちらとそろえる必要がある。
/// </remarks>
internal static class CSharpNames
{
    /// <summary>
    /// ファイルの C# の名前空間。csharp_namespace があればそれ、無ければパッケージ名から作る(voltech.v1 → Voltech.V1)。
    /// </summary>
    public static string Namespace(FileDescriptorProto file)
    {
        if (file.Options?.HasCsharpNamespace == true)
        {
            return file.Options.CsharpNamespace;
        }

        return string.Join(".", file.Package.Split('.').Select(PascalCase));
    }

    /// <summary>
    /// 生成するファイルの名前の元(app/v1/error_code.proto → ErrorCode)。protoc が Player.cs などを作るのと同じ。
    /// </summary>
    public static string FileBaseName(FileDescriptorProto file)
    {
        return PascalCase(Path.GetFileNameWithoutExtension(file.Name));
    }

    /// <summary>
    /// 名前空間をフォルダにしたもの(App.V1 → App/V1)。base_namespace を空にした protoc の出力先と同じ。
    /// </summary>
    public static string Directory(string csharpNamespace)
    {
        return csharpNamespace.Replace('.', '/');
    }

    /// <summary>
    /// フィールド名のプロパティ名(player_diff → PlayerDiff)。
    /// </summary>
    public static string PropertyName(string fieldName)
    {
        return PascalCase(fieldName);
    }

    /// <summary>
    /// メソッド名の先頭を小文字にしたもの(SignIn → signIn)。生成するフィールド名に使う。
    /// </summary>
    public static string CamelCase(string name)
    {
        return name.Length == 0 ? name : char.ToLowerInvariant(name[0]) + name[1..];
    }

    /// <summary>
    /// エラーコードの reason の定数名(PLAYER_NOT_FOUND → PlayerNotFound)。
    /// </summary>
    public static string ErrorConstantName(string reason)
    {
        return PascalCase(reason.ToLowerInvariant());
    }

    // 区切り(_)を消し、その次の文字と先頭を大文字にする。数字の後ろも大文字にする(protoc と同じ)
    private static string PascalCase(string name)
    {
        var builder = new StringBuilder(name.Length);
        var capitalizeNext = true;
        foreach (var c in name)
        {
            if (c == '_')
            {
                capitalizeNext = true;
                continue;
            }

            builder.Append(capitalizeNext ? char.ToUpperInvariant(c) : c);
            capitalizeNext = char.IsDigit(c);
        }

        return builder.ToString();
    }
}
