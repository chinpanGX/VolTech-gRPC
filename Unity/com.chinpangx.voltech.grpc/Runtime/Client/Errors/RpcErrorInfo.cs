using System.Collections.Generic;

namespace VolTech.Client
{
    /// <summary>
    /// サーバーがエラーと一緒に返す google.rpc.ErrorInfo の中身。
    /// </summary>
    public sealed class RpcErrorInfo
    {
        /// <summary>
        /// ErrorInfo の中身を受け取って作る。
        /// </summary>
        public RpcErrorInfo(string reason, string domain, IReadOnlyDictionary<string, string> metadata)
        {
            Reason = reason ?? "";
            Domain = domain ?? "";
            Metadata = metadata ?? new Dictionary<string, string>();
        }

        /// <summary>ErrorInfo.reason(例: PLAYER_NOT_FOUND)。</summary>
        public string Reason { get; }

        /// <summary>ErrorInfo.domain(例: voltech)。</summary>
        public string Domain { get; }

        /// <summary>ErrorInfo.metadata。無ければ空。</summary>
        public IReadOnlyDictionary<string, string> Metadata { get; }
    }
}
