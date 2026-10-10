using System;
using Cysharp.Threading.Tasks;
using Google.Protobuf;
using Sandbox.V1;
using VolTech.Client;

namespace Sandbox
{
    /// <summary>
    /// player_diff を SandboxState に反映する。
    /// </summary>
    public sealed class PlayerDiffApplier : IPlayerDiffApplier
    {
        private readonly SandboxState state;

        /// <summary>反映先を受け取って作る。</summary>
        public PlayerDiffApplier(SandboxState state)
        {
            this.state = state;
        }

        /// <summary>player_diff を持つレスポンスなら、入っている値だけを反映する。</summary>
        public UniTask ApplyAsync(IMessage response)
        {
            // player_diff を持つレスポンスには、protoc-gen-voltech-grpc が IHasPlayerDiff を付けている
            if (response is IHasPlayerDiff { PlayerDiff: { HasCounter: true } diff })
            {
                state.SetCounter(diff.Counter);
            }

            return UniTask.CompletedTask;
        }
    }

    /// <summary>
    /// badges を SandboxState に反映する。
    /// </summary>
    public sealed class BadgeApplier : IBadgeApplier
    {
        private readonly SandboxState state;

        /// <summary>反映先を受け取って作る。</summary>
        public BadgeApplier(SandboxState state)
        {
            this.state = state;
        }

        /// <summary>badges を持つレスポンスなら、知っている種類のバッジだけを反映する。</summary>
        public UniTask ApplyAsync(IMessage response)
        {
            if (response is IHasBadges hasBadges)
            {
                foreach (var badge in hasBadges.Badges)
                {
                    // 古いアプリは、サーバーが後から足した種類を知らないので無視する
                    if (Enum.IsDefined(typeof(BadgeType), badge.Type) && badge.Type != BadgeType.Unspecified)
                    {
                        state.SetBadge(badge);
                    }
                }
            }

            return UniTask.CompletedTask;
        }
    }

    /// <summary>
    /// notifications を SandboxState の待ち行列に積む。
    /// </summary>
    public sealed class NotificationApplier : INotificationApplier
    {
        private readonly SandboxState state;

        /// <summary>積む先を受け取って作る。</summary>
        public NotificationApplier(SandboxState state)
        {
            this.state = state;
        }

        /// <summary>notifications を持つレスポンスなら、知っている種類の通知だけを積む。</summary>
        public UniTask ApplyAsync(IMessage response)
        {
            if (response is IHasNotifications hasNotifications)
            {
                foreach (var notification in hasNotifications.Notifications)
                {
                    // 古いアプリは、サーバーが後から足した種類を知らないので無視する
                    if (notification.EventCase != Notification.EventOneofCase.None)
                    {
                        state.Enqueue(notification);
                    }
                }
            }

            return UniTask.CompletedTask;
        }
    }
}
