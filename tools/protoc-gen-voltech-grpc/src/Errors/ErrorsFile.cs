using System.Text.RegularExpressions;
using Tomlyn;
using Tomlyn.Model;

namespace ProtocGenVoltechGrpc.Errors;

/// <summary>
/// エラーの定義ファイル(errors.toml)の中身。
/// </summary>
/// <remarks>
/// 規則はサーバーの voltech-grpc-error-build と同じにする(片方だけで通る定義を作らない)。
/// </remarks>
internal sealed record ErrorsFile(string Domain, IReadOnlyList<ErrorEntry> Errors)
{
    /// <summary>基盤の予約エラーの domain。アプリの定義ファイルでは使えない。</summary>
    public const string ReservedDomain = "voltech";

    // AIP-193 の ErrorInfo.reason の上限
    private const int MaxReasonLength = 63;

    private static readonly Regex domainPattern = new("^[a-z][a-z0-9-]*(\\.[a-z][a-z0-9-]*)*$");
    private static readonly Regex reasonPattern = new("^[A-Z][A-Z0-9_]*[A-Z0-9]$");
    private static readonly string[] actions = ["stay", "retry", "home", "title", "update"];

    /// <summary>
    /// 定義ファイルの中身を読み、規則に合うか確かめる。reserved は基盤の予約エラーのファイルのとき true。
    /// </summary>
    public static ErrorsFile Parse(string source, bool reserved)
    {
        TomlTable table;
        try
        {
            table = Toml.ToModel(source);
        }
        catch (TomlException exception)
        {
            throw new GeneratorException(exception.Message);
        }

        RejectUnknownKeys(table, "", "domain", "errors");
        var domain = RequiredString(table, "domain", "");
        if (!domainPattern.IsMatch(domain))
        {
            throw new GeneratorException($"domain は小文字・数字・ハイフンを . でつないだ名前にする(例: my-app): {domain}");
        }

        if (reserved != (domain == ReservedDomain))
        {
            throw new GeneratorException(reserved
                ? $"基盤の予約エラーの domain は {ReservedDomain} にする"
                : $"domain {ReservedDomain} は基盤の予約エラーが使うので、アプリでは使えない");
        }

        var entries = new List<ErrorEntry>();
        var seen = new HashSet<string>();
        if (table.TryGetValue("errors", out var value))
        {
            if (value is not TomlTableArray errors)
            {
                throw new GeneratorException("errors は [[errors]] の表の配列にする");
            }

            foreach (var error in errors)
            {
                var entry = ParseEntry(error);
                if (!seen.Add(entry.Reason))
                {
                    throw new GeneratorException($"reason が重複している: {entry.Reason}");
                }

                entries.Add(entry);
            }
        }

        return new ErrorsFile(domain, entries);
    }

    private static ErrorEntry ParseEntry(TomlTable table)
    {
        RejectUnknownKeys(table, "[[errors]] の", "reason", "action", "client_only", "description");
        var reason = RequiredString(table, "reason", "[[errors]] の");
        if (reason.Length > MaxReasonLength || !reasonPattern.IsMatch(reason))
        {
            throw new GeneratorException($"reason は大文字のスネークケース、{MaxReasonLength}文字以内にする: {reason}");
        }

        var action = RequiredString(table, "action", $"{reason} の");
        if (!actions.Contains(action))
        {
            throw new GeneratorException($"{reason}: action は {string.Join("・", actions)} のどれかにする: {action}");
        }

        var clientOnly = false;
        if (table.TryGetValue("client_only", out var flag))
        {
            clientOnly = flag as bool? ?? throw new GeneratorException($"{reason}: client_only は true か false にする");
        }

        var description = RequiredString(table, "description", $"{reason} の");
        if (description.Trim().Length == 0)
        {
            throw new GeneratorException($"{reason}: description に用途を書く");
        }

        return new ErrorEntry(reason, action, clientOnly, description);
    }

    private static string RequiredString(TomlTable table, string key, string where)
    {
        if (!table.TryGetValue(key, out var value))
        {
            throw new GeneratorException($"{where}{key} が無い");
        }

        return value as string ?? throw new GeneratorException($"{where}{key} は文字列にする");
    }

    private static void RejectUnknownKeys(TomlTable table, string where, params string[] known)
    {
        var unknown = table.Keys.FirstOrDefault(key => !known.Contains(key));
        if (unknown != null)
        {
            throw new GeneratorException($"{where}知らない項目: {unknown}(使えるのは {string.Join("・", known)})");
        }
    }
}

/// <summary>
/// エラーの定義ファイルの1件。
/// </summary>
internal sealed record ErrorEntry(string Reason, string Action, bool ClientOnly, string Description);
