using UnityEngine;
using VolTech.Grpc.Client;

namespace Api.Unity
{
    /// <summary>
    /// アプリが前面に戻ったら ServerClock を受け取る前の状態に戻す。次に API が成功したときに、サーバー時刻で合わせ直す。
    /// </summary>
    /// <remarks>
    /// iOS・Android では、ServerClock が経過時間を測る Stopwatch が端末のスリープ中に止まり、そのままだと時刻が遅れるため。
    /// </remarks>
    public sealed class ServerClockResetter : MonoBehaviour
    {
        private ServerClock clock;

        /// <summary>
        /// clock を見張る GameObject を作る。シーンを移っても消えない。ApiCore に渡すのと同じ ServerClock を渡す。
        /// </summary>
        public static ServerClockResetter Create(ServerClock clock)
        {
            var gameObject = new GameObject(nameof(ServerClockResetter));
            DontDestroyOnLoad(gameObject);
            var resetter = gameObject.AddComponent<ServerClockResetter>();
            resetter.clock = clock;
            return resetter;
        }

        private void OnApplicationPause(bool paused)
        {
            if (!paused)
            {
                clock?.Reset();
            }
        }
    }
}
