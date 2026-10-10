using System.Threading;
using VolTech.Client.Unity;
using UnityEngine;

namespace VolTech.Client.Tracing
{
    /// <summary>
    /// 通信の記録を始める。ApiChannel.CreateCallInvoker で作る CallInvoker に、記録の Interceptor を足す。
    /// </summary>
    /// <remarks>
    /// このアセンブリ(VolTech.Client.Tracing)は、エディタと開発ビルドにだけ入る(asmdef の Define Constraints)。リリースビルドでは
    /// アセンブリごと入らないので、記録の Interceptor も足されない。Api はこのアセンブリを参照せず、こちらから登録する。
    /// </remarks>
    public static class ApiTracing
    {
        private static int installed;

        /// <summary>
        /// 記録を始める。何回呼んでも1回だけ動く。
        /// </summary>
        /// <remarks>
        /// Play と実機では、シーンを読み込む前(LifetimeScope が CallInvoker を作る前)に Unity が呼ぶ。
        /// Play していないエディタ(EditMode テスト)では、VolTech.Client.Editor が呼ぶ。
        /// 実機(開発ビルド)では、記録をつながっているエディタにも送る。
        /// </remarks>
        [RuntimeInitializeOnLoadMethod(RuntimeInitializeLoadType.BeforeSceneLoad)]
        public static void Install()
        {
            if (Interlocked.Exchange(ref installed, 1) == 1)
            {
                return;
            }

            ApiChannel.AddInterceptor(new ApiTraceInterceptor(ApiTraceStore.Shared));
            if (!Application.isEditor)
            {
                ApiTraceSender.EnsureStarted(ApiTraceStore.Shared);
            }
        }
    }
}
