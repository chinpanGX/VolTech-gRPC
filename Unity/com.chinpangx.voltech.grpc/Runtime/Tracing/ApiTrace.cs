using System;
using System.Collections.Generic;

namespace VolTech.Grpc.Client.Tracing
{
    /// <summary>
    /// 1回の通信の記録。通信の Viewer(ApiTraceWindow)が表示する。
    /// </summary>
    /// <remarks>
    /// 実機からエディタへ JsonUtility で送るので、値は public なフィールドで持つ。
    /// ApiTraceStore に入れた後は書き換えない(別のスレッドから読まれるため)。状態が変わったら、同じ Id の新しい記録で置き換える。
    /// </remarks>
    [Serializable]
    public sealed class ApiTrace
    {
        /// <summary>記録の連番。送信中の記録と、それを埋めた記録は同じ Id。</summary>
        public int Id;

        /// <summary>RPC のフルパス(例: /app.v1.GachaService/Draw)。</summary>
        public string Method;

        /// <summary>送り始めた時刻(端末の時計の、端末のタイムゾーン)。ISO 8601 の文字列。</summary>
        public string StartedAt;

        /// <summary>送り始めてから結果を受け取るまでのミリ秒。送信中は -1。</summary>
        public double DurationMs = -1;

        /// <summary>締め切り(UTC)。ISO 8601 の文字列。無ければ空。</summary>
        public string Deadline;

        /// <summary>状態。</summary>
        public ApiTraceState State;

        /// <summary>送ったヘッダー(送った順)。</summary>
        public List<ApiTraceHeader> RequestHeaders = new();

        /// <summary>送った Body(整形した JSON)。</summary>
        public string RequestBody;

        /// <summary>受け取ったヘッダー。</summary>
        public List<ApiTraceHeader> ResponseHeaders = new();

        /// <summary>受け取った Body(整形した JSON)。エラーのときは空。</summary>
        public string ResponseBody;

        /// <summary>受け取ったトレーラー。</summary>
        public List<ApiTraceHeader> Trailers = new();

        /// <summary>gRPC のステータスコードの名前(例: OK、FailedPrecondition)。送信中は空。</summary>
        public string StatusCode;

        /// <summary>ステータスの detail(サーバーのメッセージ)。</summary>
        public string StatusDetail;

        /// <summary>ErrorInfo.reason(例: ERROR_CODE_INSUFFICIENT_GEMS)。無ければ空。</summary>
        public string ErrorReason;

        /// <summary>ErrorInfo.domain。</summary>
        public string ErrorDomain;

        /// <summary>ErrorInfo.metadata。</summary>
        public List<ApiTraceHeader> ErrorMetadata = new();

        /// <summary>実機から送るときに、大きすぎる Body を切り詰めたか。</summary>
        public bool Truncated;

        /// <summary>RPC のフルパスから、サービス名を省いた短い名前(例: Draw)。</summary>
        public string ShortMethod => Method?.Substring(Method.LastIndexOf('/') + 1) ?? "";

        /// <summary>
        /// 同じ中身の複製を作る。記録を書き換えずに、状態を変えた新しい記録を作るときに使う。
        /// </summary>
        public ApiTrace Clone()
        {
            var clone = (ApiTrace)MemberwiseClone();
            clone.RequestHeaders = new List<ApiTraceHeader>(RequestHeaders);
            clone.ResponseHeaders = new List<ApiTraceHeader>(ResponseHeaders);
            clone.Trailers = new List<ApiTraceHeader>(Trailers);
            clone.ErrorMetadata = new List<ApiTraceHeader>(ErrorMetadata);
            return clone;
        }
    }

    /// <summary>
    /// 通信の状態。
    /// </summary>
    public enum ApiTraceState
    {
        /// <summary>応答を待っている。</summary>
        Pending,

        /// <summary>成功した。</summary>
        Succeeded,

        /// <summary>エラーになった。</summary>
        Failed,
    }

    /// <summary>
    /// ヘッダー・トレーラー・ErrorInfo.metadata の1項目。
    /// </summary>
    [Serializable]
    public sealed class ApiTraceHeader
    {
        /// <summary>名前。</summary>
        public string Key;

        /// <summary>値。バイナリ(-bin)は、バイト数と base64。</summary>
        public string Value;

        /// <summary>
        /// 空の項目を作る(JsonUtility が読み戻すときに使う)。
        /// </summary>
        public ApiTraceHeader()
        {
        }

        /// <summary>
        /// 名前と値を受け取って作る。
        /// </summary>
        public ApiTraceHeader(string key, string value)
        {
            Key = key;
            Value = value;
        }
    }
}
