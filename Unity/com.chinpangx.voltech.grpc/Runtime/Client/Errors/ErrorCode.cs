using System;

namespace VolTech.Client
{
    /// <summary>
    /// エラーコード。ErrorInfo の domain と reason の組で見分ける。
    /// </summary>
    /// <remarks>
    /// 定数はエラーの定義ファイル(errors.toml)から生成する。基盤の予約エラーは ReservedErrors(domain: voltech)にある。
    /// </remarks>
    public readonly struct ErrorCode : IEquatable<ErrorCode>
    {
        /// <summary>
        /// domain と reason からエラーコードを作る。
        /// </summary>
        public ErrorCode(string domain, string reason)
        {
            Domain = domain ?? "";
            Reason = reason ?? "";
        }

        /// <summary>ErrorInfo.domain(例: voltech)。</summary>
        public string Domain { get; }

        /// <summary>ErrorInfo.reason(例: PLAYER_NOT_FOUND)。</summary>
        public string Reason { get; }

        /// <summary>
        /// 基盤の予約エラーかを返す。予約エラーは handledCodes に渡しても、必ず共通エラーハンドラーが処理する。
        /// </summary>
        public bool IsReserved => Domain == ReservedErrors.Domain;

        public static bool operator ==(ErrorCode left, ErrorCode right)
        {
            return left.Equals(right);
        }

        public static bool operator !=(ErrorCode left, ErrorCode right)
        {
            return !left.Equals(right);
        }

        public bool Equals(ErrorCode other)
        {
            return string.Equals(Domain, other.Domain, StringComparison.Ordinal)
                && string.Equals(Reason, other.Reason, StringComparison.Ordinal);
        }

        public override bool Equals(object obj)
        {
            return obj is ErrorCode other && Equals(other);
        }

        public override int GetHashCode()
        {
            return HashCode.Combine(Domain, Reason);
        }

        /// <summary>
        /// ログ用の文字列(domain/reason)にする。
        /// </summary>
        public override string ToString()
        {
            return $"{Domain}/{Reason}";
        }
    }
}
