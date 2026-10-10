namespace VolTech.Client
{
    /// <summary>
    /// サーバーとやり取りするヘッダー(メタデータ)の名前。サーバーの名前(server/rust/status など)と一致させる。
    /// </summary>
    public static class ApiHeaders
    {
        /// <summary>リクエスト: アクセストークン(Bearer)。</summary>
        public const string Authorization = "authorization";

        /// <summary>リクエスト: 冪等性の対象の RPC の RequestID。</summary>
        public const string RequestId = "x-request-id";

        /// <summary>リクエスト: アプリのプラットフォーム(ios・android・windows・editor)。強制アップデートの判定に使う。</summary>
        public const string AppPlatform = "x-app-platform";

        /// <summary>リクエスト: アプリのバージョン(リリースのたびに増やす整数)。強制アップデートの判定に使う。</summary>
        public const string AppVersion = "x-app-version";

        /// <summary>リクエスト: セッションの最初に受け取ったゲーム内の日付。レスポンス: 今のゲーム内の日付。形は YYYY-MM-DD。</summary>
        public const string GameDate = "x-game-date";

        /// <summary>レスポンス: サーバー時刻(Unix 時間のミリ秒)。</summary>
        public const string ServerTime = "x-server-time";
    }
}
