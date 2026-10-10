namespace ProtocGenVoltechGrpc;

/// <summary>
/// プラグインのオプション(buf.gen.yaml の opt)。カンマ区切りの key=value で渡される。
/// </summary>
/// <remarks>
/// - package=app.v1: 生成する .proto のパッケージ。複数書ける。書かなければ全部を生成する。
///   基盤のデバイス登録(voltech.v1)は Auth が直接呼ぶので、ラッパーを作らないように絞る
/// </remarks>
internal sealed class GeneratorParameters
{
    private readonly HashSet<string> packages = new();

    private GeneratorParameters()
    {
    }

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
                default:
                    throw new GeneratorException($"知らないオプション: {key}(使えるのは package)");
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
