using System;
using System.Threading;
using UnityEngine;
using UnityEngine.Networking.PlayerConnection;

namespace VolTech.Client.Tracing
{
    /// <summary>
    /// 実機(開発ビルド)の通信の記録を、PlayerConnection でつながっているエディタに送る。
    /// </summary>
    /// <remarks>
    /// エディタがつながったときは持っている記録をすべて送り、その後は記録が足されたり埋められたりするたびに1件ずつ送る。
    /// 埋めた記録は Request を省いて送る(Request の Body を2回送らない)。
    /// つながっていなければ送らない(記録は端末のメモリにだけ残る)。
    /// </remarks>
    internal static class ApiTraceSender
    {
        private static int started;

        /// <summary>
        /// 送り始める。何回呼んでも1回だけ動く。メインスレッドから呼ぶ。
        /// </summary>
        public static void EnsureStarted(ApiTraceStore store)
        {
            if (Interlocked.Exchange(ref started, 1) == 1)
            {
                return;
            }

            // 記録は Interceptor の続き(別スレッドのことがある)で足されるので、送るのはメインスレッドに戻してからにする。
            // PlayerConnection はメインスレッドでしか使えないので、戻す先が無ければ送らない
            var mainThread = SynchronizationContext.Current;
            if (mainThread == null)
            {
                Debug.LogWarning("[ApiTrace] メインスレッドの SynchronizationContext が無いので、通信の記録をエディタに送りません。");
                return;
            }

            var connection = PlayerConnection.instance;
            connection.RegisterConnection(_ =>
            {
                // エディタに前の記録を消させてから、持っている記録をすべて送り直す
                connection.Send(ApiTraceMessages.Reset, Array.Empty<byte>());
                foreach (var trace in store.Snapshot())
                {
                    connection.Send(ApiTraceMessages.Trace, ApiTraceMessages.Encode(trace));
                }
            });
            store.Changed += trace => mainThread.Post(_ => Send(connection, trace), null);
        }

        // 結果を埋めた記録は、Request を省いて送る。送信中の記録(Request を含む)は、それより先に送ってある。
        // 送信中の記録を足したときに、つながっていなかったとしても、つながったときに持っている記録をすべて送るので届いている
        private static void Send(PlayerConnection connection, ApiTrace trace)
        {
            if (connection.isConnected)
            {
                var omitRequest = trace.State != ApiTraceState.Pending;
                connection.Send(ApiTraceMessages.Trace, ApiTraceMessages.Encode(trace, omitRequest));
            }
        }
    }
}
