namespace VolTech.Client
{
    /// <summary>
    /// エラーが起きたときにクライアントが取る行動。エラーの定義ファイル(errors.toml)の action に対応する。
    /// </summary>
    public enum ErrorAction
    {
        Unspecified = 0,
        Stay = 1,
        Retry = 2,
        Home = 3,
        Title = 4,
        Update = 5,
    }
}
