namespace ProtocGenVoltechGrpc;

/// <summary>
/// プラグインのオプション(buf.gen.yaml の opt)。カンマ区切りの key=value で渡される。
/// </summary>
/// <remarks>
/// - package=app.v1: 生成する .proto のパッケージ。複数書ける。書かなければ全部を生成する。
///   基盤のデバイス登録(voltech.v1)は Auth が直接呼ぶので、ラッパーを作らないように絞る
/// - error_code=app.v1.ErrorCode: handledCodes に使う enum の完全名。書かなければ、名前が ErrorCode の enum を探す
/// </remarks>
internal sealed class GeneratorParameters
{
    private readonly HashSet<string> packages = new();

    private GeneratorParameters()
    {
    }

    /// <summary>handledCodes に使う enum の完全名。指定が無ければ null。</summary>
    public string? ErrorCode { get; private set; }

    public static GeneratorParameters Parse(string parameter)
    {
        var result = new GeneratorParameters();
        foreach (var entry in parameter.Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries))
        {
            var separator = entry.IndexOf('=');
            if (separator < 0)
            {
                throw new GeneratorException($"オプションは key=value で書く: {entry}");
            }

            var key = entry[..separator];
            var value = entry[(separator + 1)..];
            switch (key)
            {
                case "package":
                    result.packages.Add(value);
                    break;
                case "error_code":
                    result.ErrorCode = value;
                    break;
                default:
                    throw new GeneratorException($"知らないオプション: {key}(使えるのは package・error_code)");
            }
        }

        return result;
    }

    /// <summary>
    /// このパッケージの .proto から生成するか。
    /// </summary>
    public bool Includes(string package)
    {
        return packages.Count == 0 || packages.Contains(package);
    }
}
