namespace VolTech.Grpc.Client
{
    /// <summary>
    /// RPC ごとの設定。ラッパーが .proto に合わせて持つ。
    /// </summary>
    public sealed class RpcSettings
    {
        /// <summary>
        /// RPC の名前と、冪等性の対象か・認証が要るかを受け取って作る。
        /// </summary>
        public RpcSettings(string methodName, bool isIdempotencyTarget, bool requiresAuth)
        {
            MethodName = methodName;
            IsIdempotencyTarget = isIdempotencyTarget;
            RequiresAuth = requiresAuth;
        }

        /// <summary>RPC のパス(例: /app.v1.PlayerService/SignIn)。ログに出す。</summary>
        public string MethodName { get; }

        /// <summary>
        /// 冪等性の対象(idempotency_level を書いていない)か。対象なら順番待ちをして、RequestID を付ける。
        /// </summary>
        public bool IsIdempotencyTarget { get; }

        /// <summary>認証が要る(サービスに skip_auth が付いていない)か。</summary>
        public bool RequiresAuth { get; }
    }
}
