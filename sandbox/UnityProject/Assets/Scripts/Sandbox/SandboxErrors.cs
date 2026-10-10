using Google.Protobuf.Reflection;
using Sandbox.V1;
using VolTech.Client;

namespace Sandbox
{
    /// <summary>
    /// sandbox の error_code.proto(ErrorCode)を基盤に渡す。
    /// </summary>
    public static class SandboxErrors
    {
        /// <summary>sandbox の ErrorCode から、エラーコードの名前・番号・既定の action の対応を作る。</summary>
        public static IErrorPolicyProvider CreatePolicyProvider()
        {
            return new ProtoErrorPolicyProvider(ErrorCodeReflection.Descriptor.FindTypeByName<EnumDescriptor>("ErrorCode"));
        }
    }
}
