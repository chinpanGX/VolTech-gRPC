using System;
using System.Text;
using UnityEngine;

namespace VolTech.Client.Tracing
{
    /// <summary>
    /// 実機からエディタへ、PlayerConnection で通信の記録を送るときの決まり(メッセージの ID と、送る形)。
    /// </summary>
    internal static class ApiTraceMessages
    {
        /// <summary>通信の記録1件のメッセージの ID。実機とエディタで同じ値を使う。</summary>
        public static readonly Guid Trace = new("6c1f3c0e-2b4d-4a8f-9e57-3d2a8b1c7f40");

        /// <summary>
        /// 記録を送り直す前に送る、中身の無いメッセージの ID。受け取ったエディタは、その端末の分の記録を消す。
        /// </summary>
        /// <remarks>
        /// 同じ端末からのメッセージは送った順に届くので、続けて送る記録より先に処理される。
        /// エディタの接続のイベントで消すと、先に届いた記録まで消すことがある。
        /// </remarks>
        public static readonly Guid Reset = new("0d6e2a91-5f3b-4c7e-8a14-b92f6c3d5e07");

        /// <summary>送るときの Body の上限(文字数)。超えたら切り詰め、Truncated にする。</summary>
        public const int MaxBodyLength = 256 * 1024;

        /// <summary>
        /// 記録を、送る形(JSON の UTF-8)にする。大きすぎる Body は切り詰める(元の記録は変えない)。
        /// </summary>
        /// <param name="trace">送る記録。</param>
        /// <param name="omitRequest">
        /// Request(ヘッダーと Body)を省くか。同じ Id の送信中の記録を送ってあれば、Request は変わらないので省ける。
        /// 受け取ったエディタが、手元の記録から埋める。
        /// </param>
        public static byte[] Encode(ApiTrace trace, bool omitRequest = false)
        {
            // Body を読むと JSON になり、記録のフィールドに残る(JsonUtility はそのフィールドを書き出す)
            var requestBody = omitRequest ? null : trace.RequestBody;
            var responseBody = trace.ResponseBody;
            var truncate = IsTooLong(requestBody) || IsTooLong(responseBody);
            if (!truncate && !omitRequest)
            {
                return Encoding.UTF8.GetBytes(JsonUtility.ToJson(trace));
            }

            var sending = trace.Clone();
            if (omitRequest)
            {
                sending.RequestHeaders.Clear();
                sending.RequestOmitted = true;
            }

            sending.RequestBody = Truncate(requestBody);
            sending.ResponseBody = Truncate(responseBody);
            sending.Truncated = truncate;
            return Encoding.UTF8.GetBytes(JsonUtility.ToJson(sending));
        }

        /// <summary>
        /// 受け取ったバイト列を記録に戻す。
        /// </summary>
        public static ApiTrace Decode(byte[] data)
        {
            return JsonUtility.FromJson<ApiTrace>(Encoding.UTF8.GetString(data));
        }

        private static bool IsTooLong(string body)
        {
            return body != null && body.Length > MaxBodyLength;
        }

        private static string Truncate(string body)
        {
            if (!IsTooLong(body))
            {
                return body;
            }

            // サロゲートペアの途中で切ると、UTF-8 にしたときに文字化けするので、その手前で切る
            var length = char.IsHighSurrogate(body[MaxBodyLength - 1]) ? MaxBodyLength - 1 : MaxBodyLength;
            return body.Substring(0, length) + "\n... (truncated)";
        }
    }
}
