namespace ProtocGenVoltechGrpc;

/// <summary>
/// .proto やプラグインの指定が生成できない形のとき。メッセージは protoc / buf のエラーとして表示される。
/// </summary>
internal sealed class GeneratorException(string message) : Exception(message);
