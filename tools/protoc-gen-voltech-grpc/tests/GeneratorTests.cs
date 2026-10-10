using Google.Protobuf.Compiler;
using Google.Protobuf.Reflection;

namespace ProtocGenVoltechGrpc.Tests;

public class GeneratorTests
{
    private static readonly FileDescriptorProto[] appFiles = [Protos.ErrorCode(), Protos.Player(), Protos.Gacha()];

    [Fact]
    public void GeneratesOneFilePerProtoIntoNamespaceFolder()
    {
        var response = Generate("", appFiles, "app/v1/player.proto", "app/v1/gacha.proto", "app/v1/error_code.proto");

        Assert.Equal("", response.Error);
        // error_code.proto は生成するものが無いので作らない
        Assert.Equal(["App/V1/PlayerApi.cs", "App/V1/GachaApi.cs"], response.File.Select(file => file.Name));
    }

    [Fact]
    public void EmbedsIdempotencyAndAuthFromProto()
    {
        var gacha = Content(Generate("", appFiles, "app/v1/gacha.proto"), "App/V1/GachaApi.cs");
        var player = Content(Generate("", appFiles, "app/v1/player.proto"), "App/V1/PlayerApi.cs");

        // idempotency_level を書いていない Draw だけが冪等性の対象
        Assert.Contains("new(GachaServiceMethods.Draw, isIdempotencyTarget: true, requiresAuth: true)", gacha);
        Assert.Contains("new(PlayerServiceMethods.SignIn, isIdempotencyTarget: false, requiresAuth: true)", player);
    }

    [Fact]
    public void SkipAuthServiceDoesNotRequireAuth()
    {
        var files = new[] { Protos.ErrorCode(), Protos.Device() };
        var device = Content(Generate("", files, "voltech/v1/device.proto"), "Voltech/V1/DeviceApi.cs");

        // csharp_namespace が無ければ、パッケージ名から名前空間を作る(protoc と同じ)
        Assert.Contains("namespace Voltech.V1", device);
        Assert.Contains("isIdempotencyTarget: false, requiresAuth: false", device);
    }

    [Fact]
    public void GeneratesMethodPathsAndWrapperNamedAfterService()
    {
        var player = Content(Generate("", appFiles, "app/v1/player.proto"), "App/V1/PlayerApi.cs");

        // ApiCore などクライアントの基盤の型を使うので、その名前空間を読み込む
        Assert.Contains("using VolTech.Client;", player);
        Assert.Contains("public const string SignIn = \"/app.v1.PlayerService/SignIn\";", player);
        Assert.Contains("public sealed class PlayerApi", player);
        Assert.Contains("public UniTask<ApiResult<global::App.V1.SignInResponse>> SignInAsync(", player);
        Assert.Contains("IReadOnlyList<global::App.V1.ErrorCode> handledCodes = null)", player);
    }

    [Fact]
    public void MarksResponsesWithPlayerDiffAndDefinesInterfaceWithItsType()
    {
        var response = Generate("", appFiles, "app/v1/player.proto", "app/v1/gacha.proto");

        // インターフェースは PlayerDiff を定義したファイルに、目印は player_diff を持つレスポンスのファイルに出す
        Assert.Contains("public interface IHasPlayerDiff", Content(response, "App/V1/PlayerApi.cs"));
        Assert.Contains("global::App.V1.PlayerDiff PlayerDiff { get; }", Content(response, "App/V1/PlayerApi.cs"));
        Assert.Contains("public partial class DrawResponse : IHasPlayerDiff", Content(response, "App/V1/GachaApi.cs"));
        Assert.DoesNotContain("partial class SignInResponse", Content(response, "App/V1/PlayerApi.cs"));
    }

    [Fact]
    public void PackageOptionLimitsGeneratedFiles()
    {
        var files = appFiles.Append(Protos.Device()).ToArray();

        var response = Generate("package=app.v1", files, "app/v1/player.proto", "voltech/v1/device.proto");

        Assert.Equal(["App/V1/PlayerApi.cs"], response.File.Select(file => file.Name));
    }

    [Fact]
    public void StreamingRpcIsError()
    {
        var player = Protos.Player();
        var streaming = Protos.Method("Watch", ".app.v1.SignInRequest", ".app.v1.SignInResponse", idempotent: true);
        streaming.ServerStreaming = true;
        player.Service[0].Method.Add(streaming);

        var exception = Assert.Throws<GeneratorException>(
            () => Generate("", [Protos.ErrorCode(), player], "app/v1/player.proto"));
        Assert.Contains("ストリーミング", exception.Message);
    }

    [Fact]
    public void MissingErrorCodeIsError()
    {
        var exception = Assert.Throws<GeneratorException>(
            () => Generate("", [Protos.Player()], "app/v1/player.proto"));
        Assert.Contains("error_code", exception.Message);
    }

    [Fact]
    public void UnknownOptionIsError()
    {
        var exception = Assert.Throws<GeneratorException>(
            () => Generate("packages=app.v1", appFiles, "app/v1/player.proto"));
        Assert.Contains("packages", exception.Message);
    }

    private static CodeGeneratorResponse Generate(string parameter, IEnumerable<FileDescriptorProto> files, params string[] generate)
    {
        return Generator.Generate(Protos.Request(parameter, files, generate));
    }

    private static string Content(CodeGeneratorResponse response, string name)
    {
        return response.File.Single(file => file.Name == name).Content;
    }
}
