using ProtocGenVoltechGrpc.Errors;

namespace ProtocGenVoltechGrpc.Tests;

public class ErrorsTests
{
    private const string App = """
        domain = "my-app"

        [[errors]]
        reason = "INSUFFICIENT_GEMS"
        action = "stay"
        description = "ジェムが足りない。"

        [[errors]]
        reason = "OFFLINE_ONLY"
        action = "title"
        client_only = true
        description = "クライアントだけが使う。"
        """;

    [Fact]
    public void GeneratesConstantsAndDefinitionsForAllErrors()
    {
        var code = Generate(App);

        Assert.Contains("namespace App", code);
        Assert.Contains("public static class AppErrors", code);
        Assert.Contains("public const string Domain = \"my-app\";", code);
        Assert.Contains("/// ジェムが足りない。", code);
        Assert.Contains("public static readonly global::VolTech.Client.ErrorCode InsufficientGems = new(Domain, \"INSUFFICIENT_GEMS\");", code);
        Assert.Contains("new global::VolTech.Client.ErrorDefinition(InsufficientGems, global::VolTech.Client.ErrorAction.Stay),", code);
        // クライアントだけが使うエラーも、クライアントの定数と一覧には入れる
        Assert.Contains("public static readonly global::VolTech.Client.ErrorCode OfflineOnly = new(Domain, \"OFFLINE_ONLY\");", code);
        Assert.Contains("new global::VolTech.Client.ErrorDefinition(OfflineOnly, global::VolTech.Client.ErrorAction.Title),", code);
    }

    [Fact]
    public void FileWithoutErrorsGeneratesEmptyList()
    {
        var code = Generate("domain = \"my-app\"");

        Assert.Contains("public static readonly IReadOnlyList<global::VolTech.Client.ErrorDefinition> All = new global::VolTech.Client.ErrorDefinition[]", code);
    }

    [Fact]
    public void AppCannotUseReservedDomain()
    {
        var exception = Assert.Throws<GeneratorException>(() => Generate(App.Replace("my-app", "voltech")));
        Assert.Contains("voltech", exception.Message);
    }

    [Fact]
    public void ReservedRequiresReservedDomain()
    {
        Assert.Throws<GeneratorException>(() => ErrorsFile.Parse(App, reserved: true));
        Assert.Equal("voltech", ErrorsFile.Parse(App.Replace("my-app", "voltech"), reserved: true).Domain);
    }

    [Theory]
    [InlineData("INSUFFICIENT_GEMS", "insufficient_gems")]
    [InlineData("INSUFFICIENT_GEMS", "INSUFFICIENT_")]
    [InlineData("INSUFFICIENT_GEMS", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA")]
    [InlineData("OFFLINE_ONLY", "INSUFFICIENT_GEMS")]
    [InlineData("my-app", "MyApp")]
    [InlineData("\"stay\"", "\"dialog\"")]
    [InlineData("client_only = true", "client = true")]
    [InlineData("client_only = true", "client_only = \"yes\"")]
    [InlineData("ジェムが足りない。", "")]
    [InlineData("reason = \"OFFLINE_ONLY\"", "reason = \"ALL\"")]
    [InlineData("domain = ", "domain = [")]
    public void InvalidDefinitionIsError(string from, string to)
    {
        Assert.Throws<GeneratorException>(() => Generate(App.Replace(from, to)));
    }

    private static string Generate(string source)
    {
        return ErrorsGenerator.Generate(ErrorsFile.Parse(source, reserved: false), "App", "AppErrors", "errors.toml");
    }
}
