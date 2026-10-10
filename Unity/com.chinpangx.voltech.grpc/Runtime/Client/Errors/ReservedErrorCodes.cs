namespace VolTech.Client
{
    /// <summary>
    /// 基盤の予約エラーコード(1〜999)のうち、クライアントが自分で使う番号。値はアプリの error_code.proto と同じ。
    /// </summary>
    public static class ReservedErrorCodes
    {
        public const int NetworkError = 1;
        public const int InternalError = 2;
        public const int UnknownError = 3;
        public const int ClientError = 8;

        private const int MaxReserved = 999;

        /// <summary>
        /// 予約コード(1〜999)かを返す。予約コードは handledCodes に渡しても、必ず共通エラーハンドラーが処理する。
        /// </summary>
        public static bool IsReserved(int code)
        {
            return code is >= 1 and <= MaxReserved;
        }
    }
}
