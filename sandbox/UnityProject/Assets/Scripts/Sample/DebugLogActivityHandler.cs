using UnityEngine;
using VolTech.Client;

namespace Sample
{
    /// <summary>
    /// 通信中になった・終わったことを Debug.Log に出す。IApiActivityHandler の実装例。
    /// </summary>
    /// <remarks>
    /// アプリでは、ここで通信中の表示と入力ブロックを出す・消す。サンプルでは通知が届く時点を確かめるために、ログに出すだけにする。
    /// </remarks>
    public sealed class DebugLogActivityHandler : IApiActivityHandler
    {
        /// <inheritdoc />
        public void OnActivityStarted()
        {
            Debug.Log("[Activity] started");
        }

        /// <inheritdoc />
        public void OnActivityEnded()
        {
            Debug.Log("[Activity] ended");
        }
    }
}
