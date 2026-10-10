namespace ProtocGenVoltechGrpc.Errors;

/// <summary>
/// errors サブコマンド: エラーの定義ファイル(errors.toml)から C# を生成する。
/// </summary>
/// <remarks>
/// protoc-gen-voltech-grpc errors &lt;errors.toml&gt; &lt;出力する .cs&gt; --namespace &lt;名前空間&gt; --class &lt;クラス名&gt;
/// </remarks>
internal static class ErrorsCommand
{
    public const string Name = "errors";

    private const string Usage =
        "使い方: protoc-gen-voltech-grpc errors <errors.toml> <出力する .cs> --namespace <名前空間> --class <クラス名>";

    /// <summary>
    /// 生成してファイルに書く。終了コードを返す。
    /// </summary>
    public static int Run(IReadOnlyList<string> args)
    {
        try
        {
            var (input, output, csharpNamespace, className, reserved) = ParseArgs(args);
            var file = ErrorsFile.Parse(File.ReadAllText(input), reserved);
            var code = ErrorsGenerator.Generate(file, csharpNamespace, className, Path.GetFileName(input));
            var directory = Path.GetDirectoryName(Path.GetFullPath(output));
            if (directory != null)
            {
                Directory.CreateDirectory(directory);
            }

            File.WriteAllText(output, code);
            return 0;
        }
        catch (GeneratorException exception)
        {
            Console.Error.WriteLine($"{(args.Count > 0 ? args[0] : Name)}: {exception.Message}");
            return 1;
        }
        catch (IOException exception)
        {
            Console.Error.WriteLine(exception.Message);
            return 1;
        }
    }

    private static (string Input, string Output, string Namespace, string Class, bool Reserved) ParseArgs(IReadOnlyList<string> args)
    {
        var positional = new List<string>();
        string? csharpNamespace = null;
        string? className = null;
        var reserved = false;
        for (var i = 0; i < args.Count; i++)
        {
            switch (args[i])
            {
                case "--namespace" when i + 1 < args.Count:
                    csharpNamespace = args[++i];
                    break;
                case "--class" when i + 1 < args.Count:
                    className = args[++i];
                    break;
                // 基盤の予約エラー(domain voltech)を生成する。基盤のパッケージだけが使う
                case "--reserved":
                    reserved = true;
                    break;
                default:
                    if (args[i].StartsWith("--", StringComparison.Ordinal))
                    {
                        throw new GeneratorException($"知らないオプション: {args[i]}\n{Usage}");
                    }

                    positional.Add(args[i]);
                    break;
            }
        }

        if (positional.Count != 2 || csharpNamespace == null || className == null)
        {
            throw new GeneratorException(Usage);
        }

        return (positional[0], positional[1], csharpNamespace, className, reserved);
    }
}
