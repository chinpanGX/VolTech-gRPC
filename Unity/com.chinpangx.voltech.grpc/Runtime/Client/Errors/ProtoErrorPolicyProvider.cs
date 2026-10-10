using System.Collections.Generic;
using Google.Protobuf.Reflection;
using VolTech.V1;

namespace VolTech.Client
{
    /// <summary>
    /// アプリの error_code.proto(ErrorCode の enum)から、エラーコードの名前と番号の対応と、既定の action を作る。
    /// </summary>
    /// <example>
    /// <code>new ProtoErrorPolicyProvider(ErrorCodeReflection.Descriptor.FindTypeByName&lt;EnumDescriptor&gt;("ErrorCode"))</code>
    /// </example>
    public sealed class ProtoErrorPolicyProvider : IErrorPolicyProvider
    {
        private readonly Dictionary<string, int> codesByName = new();
        private readonly Dictionary<int, string> namesByCode = new();
        private readonly Dictionary<int, ErrorAction> defaultActions = new();

        /// <summary>
        /// ErrorCode の enum の Descriptor を読んで作る。
        /// </summary>
        public ProtoErrorPolicyProvider(EnumDescriptor errorCodes)
        {
            // 起動時に1回だけ読んでおき、エラーのたびに Descriptor を引かない
            foreach (var value in errorCodes.Values)
            {
                codesByName[value.Name] = value.Number;
                namesByCode[value.Number] = value.Name;
                var action = value.GetOptions()?.GetExtension(ErrorOptionsExtensions.DefaultAction) ?? V1.ErrorAction.Unspecified;
                // 基盤の ErrorAction と voltech/v1/error_options.proto の ErrorAction は同じ番号
                defaultActions[value.Number] = (ErrorAction)(int)action;
            }
        }

        /// <summary>
        /// エラーコードの名前を番号に変える。知らない名前なら false を返す。
        /// </summary>
        public bool TryGetCode(string codeName, out int code)
        {
            return codesByName.TryGetValue(codeName, out code);
        }

        /// <summary>
        /// エラーコードの番号から名前を返す。知らない番号なら番号をそのまま文字列にする。
        /// </summary>
        public string GetName(int code)
        {
            return namesByCode.TryGetValue(code, out var name) ? name : code.ToString();
        }

        /// <summary>
        /// エラーコードの既定の action を返す。付いていないコードは Title。
        /// </summary>
        public ErrorAction GetDefaultAction(int code)
        {
            return defaultActions.TryGetValue(code, out var action) && action != ErrorAction.Unspecified
                ? action
                : ErrorAction.Title;
        }
    }
}
