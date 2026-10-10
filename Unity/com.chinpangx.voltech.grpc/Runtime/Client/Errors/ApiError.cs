using System.Collections.Generic;
using Grpc.Core;

namespace VolTech.Grpc.Client
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
            int code,
            string codeName,
            ErrorAction action,
            StatusCode statusCode,
            string message,
            IReadOnlyDictionary<string, string> metadata = null)
        {
            Code = code;
            CodeName = codeName;
            Action = action;
            StatusCode = statusCode;
            Message = message;
            Metadata = metadata ?? emptyMetadata;
        }

        /// <summary>エラーコードの番号(アプリの ErrorCode の値)。</summary>
        public int Code { get; }

        /// <summary>エラーコードの名前(例: ERROR_CODE_PLAYER_NOT_FOUND)。</summary>
        public string CodeName { get; }

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
            return new ApiError(0, "DISCARDED", ErrorAction.Unspecified, StatusCode.Cancelled, "discarded before sending")
            {
                IsDiscarded = true,
            };
        }

        /// <summary>
        /// ログ用の文字列にする。
        /// </summary>
        public override string ToString()
        {
            return $"{CodeName}({Code}) action={Action} status={StatusCode} message={Message}";
        }
    }
}
