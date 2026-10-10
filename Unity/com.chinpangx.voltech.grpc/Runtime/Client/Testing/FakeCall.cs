using Grpc.Core;

namespace VolTech.Client
{
    /// <summary>
    /// FakeCallInvoker に届いた1回の呼び出し。
    /// </summary>
    public sealed class FakeCall
    {
        /// <summary>
        /// 呼び出しの内容を受け取って作る。
        /// </summary>
        public FakeCall(string methodFullName, object request, Metadata headers)
        {
            MethodFullName = methodFullName;
            Request = request;
            Headers = headers;
        }

        /// <summary>呼ばれた RPC(例: /app.v1.PlayerService/SignIn)。</summary>
        public string MethodFullName { get; }

        /// <summary>送られたリクエスト。</summary>
        public object Request { get; }

        /// <summary>送られたヘッダー。</summary>
        public Metadata Headers { get; }

        /// <summary>
        /// ヘッダーの値を返す。無ければ null を返す。
        /// </summary>
        public string GetHeader(string key)
        {
            return Headers.GetValue(key);
        }
    }
}
