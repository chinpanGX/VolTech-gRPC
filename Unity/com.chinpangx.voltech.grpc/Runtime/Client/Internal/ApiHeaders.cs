namespace VolTech.Client
{
    /// <summary>
    /// サーバーとやり取りするヘッダー(メタデータ)の名前。サーバーの名前(server/rust/status など)と一致させる。
    /// </summary>
    internal static class ApiHeaders
    {
        /// <summary>リクエスト: アクセストークン(Bearer)。</summary>
        internal const string Authorization = "authorization";

        /// <summary>リクエスト: 冪等性の対象の RPC の RequestID。</summary>
        internal const string RequestId = "x-request-id";

        /// <summary>リクエスト: アプリのプラットフォーム(ios・android・windows・editor)。強制アップデートの判定に使う。</summary>
        internal const string AppPlatform = "x-app-platform";

        /// <summary>リクエスト: アプリのバージョン(リリースのたびに増やす整数)。強制アップデートの判定に使う。</summary>
        internal const string AppVersion = "x-app-version";

        /// <summary>リクエスト: セッションの最初に受け取ったゲーム内の日付。レスポンス: 今のゲーム内の日付。形は YYYY-MM-DD。</summary>
        internal const string GameDate = "x-game-date";

        /// <summary>レスポンス: サーバー時刻(Unix 時間のミリ秒)。</summary>
        internal const string ServerTime = "x-server-time";
    }
}
