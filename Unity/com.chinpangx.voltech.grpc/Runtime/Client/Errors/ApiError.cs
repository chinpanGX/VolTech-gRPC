using System.Collections.Generic;
using Grpc.Core;

namespace VolTech.Client
{
    /// <summary>
    /// API の呼び出しの失敗。受け取ったエラーをエラーコードに変換したもの。
    /// </summary>
    public sealed class ApiError
    {
        private static readonly IReadOnlyDictionary<string, string> emptyMetadata = new Dictionary<string, string>();

        /// <summary>
        /// エラーの内容を受け取って作る。
        /// </summary>
        public ApiError(
            ErrorCode code,
            ErrorAction action,
            StatusCode statusCode,
            string message,
            IReadOnlyDictionary<string, string> metadata = null)
        {
            Code = code;
            Action = action;
            StatusCode = statusCode;
            Message = message;
            Metadata = metadata ?? emptyMetadata;
        }

        /// <summary>エラーコード(domain と reason の組。例: sandbox/PLAYER_NOT_FOUND)。</summary>
        public ErrorCode Code { get; }

        /// <summary>エラーの後に取る行動。</summary>
        public ErrorAction Action { get; }

        /// <summary>受け取った gRPC のステータス。クライアント内部のエラーでは Unknown。</summary>
        public StatusCode StatusCode { get; }

        /// <summary>ログ用のメッセージ。ユーザーには見せない。</summary>
        public string Message { get; }

        /// <summary>
        /// サーバーが ErrorInfo.metadata に入れた表示用の値(例: MAINTENANCE の終了予定 end_at)。無ければ空。
        /// </summary>
        public IReadOnlyDictionary<string, string> Metadata { get; }

        /// <summary>
        /// タイトル・ホームに戻ったために、送らずに捨てたリクエストのとき true。
        /// </summary>
        public bool IsDiscarded { get; private set; }

        /// <summary>
        /// 送らずに捨てたリクエストの失敗を作る。
        /// </summary>
        public static ApiError Discarded()
        {
            return new ApiError(
                new ErrorCode(ReservedErrors.Domain, "DISCARDED"),
                ErrorAction.Unspecified,
                StatusCode.Cancelled,
                "discarded before sending")
            {
                IsDiscarded = true,
            };
        }

        /// <summary>
        /// ログ用の文字列にする。
        /// </summary>
        public override string ToString()
        {
            return $"{Code} action={Action} status={StatusCode} message={Message}";
        }
    }
}
