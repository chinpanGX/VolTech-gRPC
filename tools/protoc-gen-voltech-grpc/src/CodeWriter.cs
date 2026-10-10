using System.Text;

namespace ProtocGenVoltechGrpc;

/// <summary>
/// インデントを付けながら C# のコードを書く。
/// </summary>
internal sealed class CodeWriter
{
    private const string Indent = "    ";

    private readonly StringBuilder builder = new();
    private int depth;

    /// <summary>
    /// 1行書く。空文字なら空行にする(行末に空白を残さない)。
    /// </summary>
    public CodeWriter Line(string text = "")
    {
        if (text.Length > 0)
        {
            for (var i = 0; i < depth; i++)
            {
                builder.Append(Indent);
            }

            builder.Append(text);
        }

        builder.Append('\n');
        return this;
    }

    /// <summary>
    /// { を書いてインデントを1段下げる。戻り値を Dispose すると } を書いて戻す。
    /// </summary>
    public IDisposable Block(string header)
    {
        Line(header);
        Line("{");
        depth++;
        return new BlockEnd(this);
    }

    /// <summary>
    /// XML ドキュメントコメントの summary を書く。lines が空なら何も書かない。
    /// </summary>
    public CodeWriter Summary(IReadOnlyList<string> lines)
    {
        if (lines.Count == 0)
        {
            return this;
        }

        Line("/// <summary>");
        foreach (var line in lines)
        {
            Line(line.Length == 0 ? "///" : "/// " + EscapeXml(line));
        }

        Line("/// </summary>");
        return this;
    }

    public override string ToString()
    {
        return builder.ToString();
    }

    private static string EscapeXml(string text)
    {
        return text.Replace("&", "&amp;").Replace("<", "&lt;").Replace(">", "&gt;");
    }

    private sealed class BlockEnd(CodeWriter writer) : IDisposable
    {
        public void Dispose()
        {
            writer.depth--;
            writer.Line("}");
        }
    }
}
