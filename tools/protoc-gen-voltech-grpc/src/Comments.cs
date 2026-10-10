using Google.Protobuf.Reflection;

namespace ProtocGenVoltechGrpc;

/// <summary>
/// .proto に書いたコメント(SourceCodeInfo)を引き、生成するコードの doc コメントにする。
/// </summary>
internal sealed class Comments
{
    // FileDescriptorProto / ServiceDescriptorProto の中でのフィールド番号。SourceCodeInfo の path はこの番号で辿る
    private const int ServiceField = FileDescriptorProto.ServiceFieldNumber;
    private const int MethodField = ServiceDescriptorProto.MethodFieldNumber;

    private readonly Dictionary<string, string> leadingComments = new();

    public Comments(FileDescriptorProto file)
    {
        if (file.SourceCodeInfo == null)
        {
            return;
        }

        foreach (var location in file.SourceCodeInfo.Location)
        {
            if (location.HasLeadingComments)
            {
                leadingComments[Key(location.Path)] = location.LeadingComments;
            }
        }
    }

    /// <summary>
    /// サービスの前に書いたコメントを行ごとに返す。無ければ空。
    /// </summary>
    public IReadOnlyList<string> ForService(int serviceIndex)
    {
        return Lines(Key([ServiceField, serviceIndex]));
    }

    /// <summary>
    /// RPC の前に書いたコメントを行ごとに返す。無ければ空。
    /// </summary>
    public IReadOnlyList<string> ForMethod(int serviceIndex, int methodIndex)
    {
        return Lines(Key([ServiceField, serviceIndex, MethodField, methodIndex]));
    }

    private IReadOnlyList<string> Lines(string key)
    {
        if (!leadingComments.TryGetValue(key, out var comment))
        {
            return [];
        }

        // protoc は「// 」の後ろを、先頭の空白と最後の改行を付けたまま渡してくる
        return comment.TrimEnd('\n').Split('\n').Select(line => line.Trim()).ToList();
    }

    private static string Key(IEnumerable<int> path)
    {
        return string.Join(",", path);
    }
}
