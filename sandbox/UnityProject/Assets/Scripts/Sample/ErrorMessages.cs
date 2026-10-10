using System;
using System.Collections.Generic;
using System.Globalization;
using Sandbox.V1;
using VolTech.Client;

namespace Sample
{
    /// <summary>
    /// エラーコードからダイアログの文言を作る。
    /// </summary>
    /// <remarks>
    /// 文言はエラーコード(ErrorInfo の domain と reason の組)で引く。実際のアプリでは、domain と reason から作った文言キー
    /// (例: sandbox/INSUFFICIENT_GEMS → error.sandbox.insufficient_gems)でローカライズの表を引く。
    /// サンプルは日本語のフォントを入れていないので、英語で書く。
    /// </remarks>
    public static class ErrorMessages
    {
        private const string MaintenanceEndAtKey = "end_at";

        private static readonly Dictionary<ErrorCode, string> messages = new()
        {
            [ReservedErrors.NetworkError] = "Could not connect to the server.",
            [ReservedErrors.InternalError] = "A server error occurred.",
            [ReservedErrors.UnknownError] = "An unexpected error occurred.",
            [ReservedErrors.Maintenance] = "The server is under maintenance.",
            [ReservedErrors.UpdateRequired] = "A new version is available. Please update the app.",
            [ReservedErrors.AccountSuspended] = "This account is suspended.",
            [ReservedErrors.DateChanged] = "The date has changed. Returning to the title.",
            [ReservedErrors.ClientError] = "An error occurred in the app.",
            [SandboxErrors.InsufficientGems] = "Not enough gems.",
            [SandboxErrors.FreeDrawUsed] = "You already used today's free draw.",
        };

        /// <summary>
        /// error の文言を返す。知らないコードは UNKNOWN_ERROR の文言にする。
        /// </summary>
        public static string For(ApiError error)
        {
            if (!messages.TryGetValue(error.Code, out var message))
            {
                message = messages[ReservedErrors.UnknownError];
            }

            // メンテナンスの終了予定(UTC の RFC 3339)は、端末のタイムゾーンで出す
            if (error.Metadata.TryGetValue(MaintenanceEndAtKey, out var endAt) &&
                DateTimeOffset.TryParse(endAt, CultureInfo.InvariantCulture, DateTimeStyles.None, out var end))
            {
                message += $"\nScheduled to end at {end.ToLocalTime():yyyy-MM-dd HH:mm}.";
            }

            return message;
        }
    }
}
