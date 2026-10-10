using System.Collections.Generic;
using Sandbox.V1;

namespace Sandbox
{
    /// <summary>
    /// サーバーから届いた通知(出来事)の待ち行列。画面が都合のよいとき(演出の後など)に取り出して出す。
    /// </summary>
    public sealed class NotificationQueue
    {
        private readonly Queue<Notification> queue = new();

        /// <summary>待っている通知の数。</summary>
        public int Count => queue.Count;

        /// <summary>
        /// 通知を積む。
        /// </summary>
        public void Enqueue(Notification notification)
        {
            queue.Enqueue(notification);
        }

        /// <summary>
        /// いちばん古い通知を取り出す。無ければ false。
        /// </summary>
        public bool TryDequeue(out Notification notification)
        {
            return queue.TryDequeue(out notification);
        }

        /// <summary>
        /// 待っている通知をすべて捨てる。タイトルに戻ったときに呼ぶ。
        /// </summary>
        public void Clear()
        {
            queue.Clear();
        }
    }
}
