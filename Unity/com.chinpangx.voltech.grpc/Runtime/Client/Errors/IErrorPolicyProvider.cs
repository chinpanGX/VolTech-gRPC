namespace VolTech.Grpc.Client
{
    /// <summary>
    /// エラーコードの名前と番号の対応と、既定の action を渡す。アプリの error_code.proto から作る。
    /// </summary>
    public interface IErrorPolicyProvider
    {
        /// <summary>
        /// ErrorInfo.reason(エラーコードの名前)を番号に変える。知らない名前なら false を返す。
        /// </summary>
        public bool TryGetCode(string codeName, out int code);

        /// <summary>
        /// エラーコードの番号から名前を返す。知らない番号なら番号をそのまま文字列にする。
        /// </summary>
        public string GetName(int code);

        /// <summary>
        /// エラーコードの既定の action を返す。付いていないコードは Title。
        /// </summary>
        public ErrorAction GetDefaultAction(int code);
    }
}
