using UnityEditor;
using UnityEditor.Networking.PlayerConnection;
using UnityEngine.Networking.PlayerConnection;
using VolTech.Client.Tracing;

namespace VolTech.Client.Editor
{
    /// <summary>
    /// PlayerConnection でつながっている実機(開発ビルド)から、通信の記録を受け取る。
    /// </summary>
    /// <remarks>
    /// EditorConnection は、呼び出し先が UnityEngine.Object のメソッドでないと登録できない(静的なメソッドやラムダは
    /// 「does not derive from UnityEngine.Object」になる)ので、エディタの中だけで生きる ScriptableSingleton にする。
    /// 接続先は、Console・Profiler と同じ Unity の接続先の選択で決まる。
    /// </remarks>
    internal sealed class GrpcMonitorReceiver : ScriptableSingleton<GrpcMonitorReceiver>
    {
        private const string ClearOnPlayKey = "VolTech.Client.Tracing.ClearOnPlay";

        /// <summary>接続先の端末から受け取った通信の記録。</summary>
        public static ApiTraceStore Store { get; } = new();

        /// <summary>
        /// Play を始めるときに、エディタの通信の記録を消すか。エディタの設定(EditorPrefs)に残す。
        /// </summary>
        public static bool ClearOnPlay
        {
            get => EditorPrefs.GetBool(ClearOnPlayKey, true);
            set => EditorPrefs.SetBool(ClearOnPlayKey, value);
        }

        // スクリプトを読み込むたびに、受け取りを始める(ウィンドウを開いていなくても受け取る)。
        // あわせて、Play していないとき(EditMode テストなど)のエディタの通信も記録する(Play と実機では ApiTracing が自分で始める)
        [InitializeOnLoadMethod]
        private static void Start()
        {
            _ = instance;
            ApiTracing.Install();
            EditorApplication.playModeStateChanged -= OnPlayModeStateChanged;
            EditorApplication.playModeStateChanged += OnPlayModeStateChanged;
        }

        // ドメインのリロードを切っているときは記録が残るので、Play を始める前に消す。
        // Play を始めた後(EnteredPlayMode)だと、最初のシーンの初期化で送った通信の記録まで消してしまう。
        // ウィンドウを閉じていても効くように、ウィンドウではなくここで行う
        private static void OnPlayModeStateChanged(PlayModeStateChange change)
        {
            if (change == PlayModeStateChange.ExitingEditMode && ClearOnPlay)
            {
                ApiTraceStore.Shared.Clear();
            }
        }

        private void OnEnable()
        {
            var connection = EditorConnection.instance;
            connection.Initialize();
            connection.Register(ApiTraceMessages.Trace, OnTrace);
            connection.Register(ApiTraceMessages.Reset, OnReset);
        }

        private void OnDisable()
        {
            var connection = EditorConnection.instance;
            connection.Unregister(ApiTraceMessages.Trace, OnTrace);
            connection.Unregister(ApiTraceMessages.Reset, OnReset);
        }

        // 新しくつながった端末は、持っている記録をすべて送り直す前に Reset を送ってくる。前の端末の記録と混ざらないように消す。
        // 接続のイベントで消すと、それより先に届いた記録まで消すことがある(届く順が決まっていない)
        private void OnReset(MessageEventArgs args)
        {
            Store.Clear();
        }

        // 同じ Id の記録は置き換わる(「送信中」が「成功」・「エラー」に変わる)
        private void OnTrace(MessageEventArgs args)
        {
            Store.Put(ApiTraceMessages.Decode(args.data));
        }
    }
}
