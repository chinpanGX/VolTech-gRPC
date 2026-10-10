using Google.Protobuf;
using Google.Protobuf.Compiler;
using ProtocGenVoltechGrpc;
using ProtocGenVoltechGrpc.Errors;

// errors サブコマンド: エラーの定義ファイルから C# を生成する(protoc は引数を渡さないので、プラグインの動きとは区別できる)
if (args.Length > 0 && args[0] == ErrorsCommand.Name)
{
    return ErrorsCommand.Run(args[1..]);
}

// protoc / buf のプラグイン: 標準入力で CodeGeneratorRequest を受け取り、標準出力に CodeGeneratorResponse を返す。
// 標準出力は応答にしか使えないので、ログやエラーは CodeGeneratorResponse.Error か標準エラーに出す
var request = CodeGeneratorRequest.Parser.ParseFrom(Console.OpenStandardInput());
CodeGeneratorResponse response;
try
{
    response = Generator.Generate(request);
}
catch (GeneratorException exception)
{
    response = new CodeGeneratorResponse { Error = exception.Message };
}

using var output = Console.OpenStandardOutput();
response.WriteTo(output);
return 0;
