namespace VolTech.Client
{
    /// <summary>
    /// エラーコードと、その既定の action。エラーの定義ファイル(errors.toml)から生成した一覧(All)に入る。
    /// </summary>
    public readonly struct ErrorDefinition
    {
        /// <summary>
        /// エラーコードと既定の action から作る。
        /// </summary>
        public ErrorDefinition(ErrorCode code, ErrorAction defaultAction)
        {
            Code = code;
            DefaultAction = defaultAction;
        }

        /// <summary>エラーコード。</summary>
        public ErrorCode Code { get; }

        /// <summary>既定の action。サーバーが metadata["action"] で上書きしなければ、これを使う。</summary>
        public ErrorAction DefaultAction { get; }
    }
}
