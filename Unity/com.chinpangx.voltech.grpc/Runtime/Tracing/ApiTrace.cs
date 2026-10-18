using System;
using System.Collections.Generic;
using UnityEngine;

namespace VolTech.Client.Tracing
{
    /// <summary>
    /// 1回の通信の記録。gRPC Monitor(GrpcMonitorWindow)が表示する。
    /// </summary>
    /// <remarks>
    /// 実機からエディタへ JsonUtility で送るので、値は public なフィールドで持つ。
    /// ApiTraceStore に入れた後は書き換えない(別のスレッドから読まれるため)。状態が変わったら、同じ Id の新しい記録で置き換える。
    /// ただし Body は、通信を遅らせないように、読まれたときに初めて JSON にして残す(SetRequestBody・SetResponseBody)。
    /// 別のスレッドが同時に読んでも、同じ文字列ができるだけなので壊れない。
    /// </remarks>
    [Serializable]
    internal sealed class ApiTrace
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

        /// <summary>送った Body(JSON)。</summary>
        public string RequestBody
        {
            get => requestBody ??= requestJson?.Value;
            set
            {
                requestBody = value;
                requestJson = null;
            }
        }

        /// <summary>
        /// 実機から送るときに、Request(ヘッダーと Body)を省いたか。同じ Id の送信中の記録で、もう送ってあるため。
        /// エディタは、受け取ったときに手元の記録から埋める(ApiTraceStore.Put)。埋められなければ true のまま残る。
        /// </summary>
        public bool RequestOmitted;

        /// <summary>受け取ったヘッダー。</summary>
        public List<ApiTraceHeader> ResponseHeaders = new();

        /// <summary>受け取った Body(JSON)。エラーのときは空。</summary>
        public string ResponseBody
        {
            get => responseBody ??= responseJson?.Value;
            set
            {
                responseBody = value;
                responseJson = null;
            }
        }

        /// <summary>受け取ったトレーラー。</summary>
        public List<ApiTraceHeader> Trailers = new();

        /// <summary>gRPC のステータスコードの名前(例: OK、FailedPrecondition)。送信中は空。</summary>
        public string StatusCode;

        /// <summary>ステータスの detail(サーバーのメッセージ)。</summary>
        public string StatusDetail;

        /// <summary>ErrorInfo.reason(例: INSUFFICIENT_GEMS)。無ければ空。</summary>
        public string ErrorReason;

        /// <summary>ErrorInfo.domain(例: voltech)。</summary>
        public string ErrorDomain;

        /// <summary>ErrorInfo.metadata。</summary>
        public List<ApiTraceHeader> ErrorMetadata = new();

        /// <summary>実機から送るときに、大きすぎる Body を切り詰めたか。</summary>
        public bool Truncated;

        // Body は JsonUtility で送るので、文字列はシリアライズするフィールドに持つ。JSON にする前の値(Lazy)は送らない。
        // Lazy は複製(Clone)と共有するので、送信中の記録と、それを埋めた記録で、同じ Body を2回 JSON にしない
        [SerializeField]
        private string requestBody;

        [SerializeField]
        private string responseBody;

        [NonSerialized]
        private Lazy<string> requestJson;

        [NonSerialized]
        private Lazy<string> responseJson;

        /// <summary>RPC のフルパスから、サービス名を省いた短い名前(例: Draw)。</summary>
        public string ShortMethod => Method?.Substring(Method.LastIndexOf('/') + 1) ?? "";

        /// <summary>
        /// 送った Body を、読まれたときに JSON にするように入れる。
        /// </summary>
        internal void SetRequestBody(Lazy<string> json)
        {
            requestBody = null;
            requestJson = json;
        }

        /// <summary>
        /// 受け取った Body を、読まれたときに JSON にするように入れる。
        /// </summary>
        internal void SetResponseBody(Lazy<string> json)
        {
            responseBody = null;
            responseJson = json;
        }

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
    internal enum ApiTraceState
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
    internal sealed class ApiTraceHeader
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
