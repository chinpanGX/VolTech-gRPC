namespace VolTech.Client
{
    /// <summary>
    /// エラーが起きたときにクライアントが取る行動。値は voltech/v1/error_options.proto の ErrorAction と同じ。
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
