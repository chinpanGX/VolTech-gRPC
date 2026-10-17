namespace VolTech.Client
{
    /// <summary>
    /// API の呼び出しが進行中になった・進行中でなくなったことを受け取る。アプリが実装し、通信中の表示や入力ブロックに使う。
    /// </summary>
    /// <remarks>
    /// 呼び出し1回ごとではなく、進行中の呼び出しの数が 0 から 1 になったときに開始を、1 から 0 になったときに終了を呼ぶ。
    /// 進行中の範囲は、順番待ち・再認証と再送・エラーハンドラー・共通レスポンス項目の反映を含み、画面の遷移の前に終わる。
    /// エラーハンドラーが入力を受け付ける UI を出すなら、アプリはその UI を通信中の表示より手前に出すか、入力ブロックを外す。
    /// 基盤は開始を登録した順に、終了を逆の順に呼ぶ。例外は基盤がログに出して、残りのハンドラーを呼ぶ。
    /// </remarks>
    public interface IApiActivityHandler
    {
        /// <summary>
        /// 進行中の呼び出しが無い状態から、呼び出しが始まった。
        /// </summary>
        public void OnActivityStarted();

        /// <summary>
        /// 進行中の呼び出しがすべて終わった。
        /// </summary>
        public void OnActivityEnded();
    }
}
