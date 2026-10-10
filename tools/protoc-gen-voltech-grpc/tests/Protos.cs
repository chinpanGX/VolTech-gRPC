using Google.Protobuf;
using Google.Protobuf.Compiler;
using Google.Protobuf.Reflection;

namespace ProtocGenVoltechGrpc.Tests;

/// <summary>
/// テストに使う .proto(FileDescriptorProto)と CodeGeneratorRequest を組み立てる。
/// </summary>
internal static class Protos
{
    /// <summary>
    /// app/v1/player.proto 相当: PlayerService(SignIn は IDEMPOTENT)と、player_diff の型 PlayerDiff。
    /// </summary>
    public static FileDescriptorProto Player()
    {
        var file = File("app/v1/player.proto", "app.v1", csharpNamespace: "App.V1");
        file.MessageType.Add(Message("SignInRequest"));
        file.MessageType.Add(Message("SignInResponse"));
        file.MessageType.Add(Message("PlayerDiff"));
        file.Service.Add(Service("PlayerService", Method("SignIn", "SignInRequest", "SignInResponse", idempotent: true)));
        return file;
    }

    /// <summary>
    /// app/v1/gacha.proto 相当: GachaService(Draw は冪等性の対象)と、player_diff を持つ DrawResponse。
    /// </summary>
    public static FileDescriptorProto Gacha()
    {
        var file = File("app/v1/gacha.proto", "app.v1", csharpNamespace: "App.V1");
        file.Dependency.Add("app/v1/player.proto");
        file.MessageType.Add(Message("DrawRequest"));
        var response = Message("DrawResponse");
        response.Field.Add(new FieldDescriptorProto
        {
            Name = "player_diff",
            Number = 1,
            Type = FieldDescriptorProto.Types.Type.Message,
            TypeName = ".app.v1.PlayerDiff",
            Label = FieldDescriptorProto.Types.Label.Optional,
        });
        file.MessageType.Add(response);
        file.Service.Add(Service("GachaService", Method("Draw", "DrawRequest", "DrawResponse", idempotent: false)));
        return file;
    }

    /// <summary>
    /// app/v1/rarity.proto 相当: enum だけで、生成するものが無いファイル。
    /// </summary>
    public static FileDescriptorProto EnumOnly()
    {
        var file = File("app/v1/rarity.proto", "app.v1", csharpNamespace: "App.V1");
        var rarity = new EnumDescriptorProto { Name = "Rarity" };
        rarity.Value.Add(new EnumValueDescriptorProto { Name = "RARITY_UNSPECIFIED", Number = 0 });
        file.EnumType.Add(rarity);
        return file;
    }

    /// <summary>
    /// voltech/v1/device.proto 相当: skip_auth の DeviceService。パッケージ名から名前空間を作ることを確かめるため、csharp_namespace は書かない。
    /// </summary>
    public static FileDescriptorProto Device()
    {
        var file = File("voltech/v1/device.proto", "voltech.v1", csharpNamespace: null);
        file.MessageType.Add(Message("RegisterRequest"));
        file.MessageType.Add(Message("RegisterResponse"));
        var service = Service("DeviceService", Method("Register", "RegisterRequest", "RegisterResponse", idempotent: true));
        service.Options = SkipAuthOptions();
        file.Service.Add(service);
        return file;
    }

    /// <summary>
    /// files を全部渡し、generate のファイルを生成させるリクエストを作る。
    /// </summary>
    public static CodeGeneratorRequest Request(string parameter, IEnumerable<FileDescriptorProto> files, params string[] generate)
    {
        var request = new CodeGeneratorRequest { Parameter = parameter };
        request.ProtoFile.AddRange(files);
        request.FileToGenerate.AddRange(generate);
        return request;
    }

    public static MethodDescriptorProto Method(string name, string input, string output, bool idempotent)
    {
        var method = new MethodDescriptorProto { Name = name, InputType = input, OutputType = output };
        if (idempotent)
        {
            method.Options = new MethodOptions { IdempotencyLevel = MethodOptions.Types.IdempotencyLevel.Idempotent };
        }

        return method;
    }

    private static FileDescriptorProto File(string name, string package, string? csharpNamespace)
    {
        var file = new FileDescriptorProto { Name = name, Package = package, Syntax = "proto3" };
        if (csharpNamespace != null)
        {
            file.Options = new Google.Protobuf.Reflection.FileOptions { CsharpNamespace = csharpNamespace };
        }

        return file;
    }

    private static DescriptorProto Message(string name)
    {
        return new DescriptorProto { Name = name };
    }

    // 型名は .proto の完全名にする(protoc が渡す形)
    private static ServiceDescriptorProto Service(string name, params MethodDescriptorProto[] methods)
    {
        var service = new ServiceDescriptorProto { Name = name };
        foreach (var method in methods)
        {
            var package = name == "DeviceService" ? "voltech.v1" : "app.v1";
            method.InputType = $".{package}.{method.InputType}";
            method.OutputType = $".{package}.{method.OutputType}";
            service.Method.Add(method);
        }

        return service;
    }

    // skip_auth(50002)= true を、protoc が渡すのと同じく未知のフィールドとして持つ ServiceOptions
    private static ServiceOptions SkipAuthOptions()
    {
        using var stream = new MemoryStream();
        var output = new CodedOutputStream(stream);
        output.WriteTag(50002, WireFormat.WireType.Varint);
        output.WriteBool(true);
        output.Flush();
        return ServiceOptions.Parser.ParseFrom(stream.ToArray());
    }
}
