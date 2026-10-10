using System.Collections.Generic;
using Sandbox.V1;

namespace Sandbox
{
    /// <summary>
    /// 共通レスポンス項目(player_diff・badges・notifications)を反映した先。sandbox では1つにまとめて持つ。
    /// </summary>
    public sealed class SandboxState
    {
        private readonly Dictionary<BadgeType, Badge> badges = new();
        private readonly Queue<Notification> notifications = new();

        /// <summary>サーバーから受け取った最新のカウンター。まだ受け取っていなければ null。</summary>
        public long? Counter { get; private set; }

        /// <summary>種類ごとの最新のバッジ。</summary>
        public IReadOnlyDictionary<BadgeType, Badge> Badges => badges;

        /// <summary>まだ出していない通知。</summary>
        public IReadOnlyCollection<Notification> Notifications => notifications;

        /// <summary>カウンターを変更後の値で置き換える。</summary>
        public void SetCounter(long counter)
        {
            Counter = counter;
        }

        /// <summary>バッジを種類ごとに置き換える。</summary>
        public void SetBadge(Badge badge)
        {
            badges[badge.Type] = badge;
        }

        /// <summary>通知を待ち行列に積む。</summary>
        public void Enqueue(Notification notification)
        {
            notifications.Enqueue(notification);
        }
    }
}
