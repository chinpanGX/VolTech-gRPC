using Google.Protobuf.Reflection;

namespace ProtocGenVoltechGrpc;

/// <summary>
/// リクエストに含まれる全ファイルのメッセージと enum を、.proto の完全名(.app.v1.SignInRequest)から引く。
/// </summary>
internal sealed class TypeIndex
{
    private readonly Dictionary<string, string> csharpNames = new();
    private readonly Dictionary<string, FileDescriptorProto> files = new();

    public TypeIndex(IEnumerable<FileDescriptorProto> protoFiles)
    {
        foreach (var file in protoFiles)
        {
            var protoPrefix = file.Package.Length == 0 ? "" : "." + file.Package;
            var csharpPrefix = "global::" + CSharpNames.Namespace(file);
            foreach (var message in file.MessageType)
            {
                AddMessage(file, message, protoPrefix, csharpPrefix);
            }

            foreach (var enumType in file.EnumType)
            {
                AddEnum(file, enumType, protoPrefix, csharpPrefix);
            }
        }
    }

    /// <summary>
    /// メッセージか enum の C# の型名(global:: 付き)を返す。
    /// </summary>
    public string CSharpName(string protoFullName)
    {
        return csharpNames.TryGetValue(protoFullName, out var name)
            ? name
            : throw new GeneratorException($"{protoFullName} が見つからない");
    }

    /// <summary>
    /// メッセージか enum を定義しているファイルを返す。
    /// </summary>
    public FileDescriptorProto FileOf(string protoFullName)
    {
        return files.TryGetValue(protoFullName, out var file)
            ? file
            : throw new GeneratorException($"{protoFullName} が見つからない");
    }

    // 入れ子のメッセージは、protoc の C# 生成と同じく Outer.Types.Inner になる
    private void AddMessage(FileDescriptorProto file, DescriptorProto message, string protoPrefix, string csharpPrefix)
    {
        var protoName = $"{protoPrefix}.{message.Name}";
        var csharpName = $"{csharpPrefix}.{message.Name}";
        csharpNames[protoName] = csharpName;
        files[protoName] = file;

        foreach (var nested in message.NestedType)
        {
            AddMessage(file, nested, protoName, csharpName + ".Types");
        }

        foreach (var nested in message.EnumType)
        {
            AddEnum(file, nested, protoName, csharpName + ".Types");
        }
    }

    private void AddEnum(FileDescriptorProto file, EnumDescriptorProto enumType, string protoPrefix, string csharpPrefix)
    {
        var protoName = $"{protoPrefix}.{enumType.Name}";
        var csharpName = $"{csharpPrefix}.{enumType.Name}";
        csharpNames[protoName] = csharpName;
        files[protoName] = file;
    }
}
