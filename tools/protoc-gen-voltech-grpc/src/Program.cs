using Google.Protobuf;
using Google.Protobuf.Compiler;
using ProtocGenVoltechGrpc;

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
