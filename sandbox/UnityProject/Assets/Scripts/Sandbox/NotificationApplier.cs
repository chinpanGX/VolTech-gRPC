using Cysharp.Threading.Tasks;
using Google.Protobuf;
using Sandbox.V1;
using VolTech.Client;

namespace Sandbox
{
    /// <summary>
    /// レスポンスの notifications を NotificationQueue に積む。通知を出すのは画面。
    /// </summary>
    public sealed class NotificationApplier : INotificationApplier
    {
        private readonly NotificationQueue queue;

        /// <summary>
        /// 積む先を受け取って作る。
        /// </summary>
        public NotificationApplier(NotificationQueue queue)
        {
            this.queue = queue;
        }

        /// <summary>
        /// レスポンスに notifications があれば、届いた順に積む。
        /// </summary>
        public UniTask ApplyAsync(IMessage response)
        {
            // notifications を持つレスポンスには、protoc-gen-voltech-grpc が IHasNotifications を付けている
            if (response is IHasNotifications hasNotifications)
            {
                foreach (var notification in hasNotifications.Notifications)
                {
                    // 古いアプリは、サーバーが後から足した種類を知らない(oneof が空になる)ので無視する
                    if (notification.EventCase != Notification.EventOneofCase.None)
                    {
                        queue.Enqueue(notification);
                    }
                }
            }

            return UniTask.CompletedTask;
        }
    }
}
