using System;
using Cysharp.Threading.Tasks;
using Google.Protobuf;
using Sandbox.V1;
using VolTech.Client;

namespace Sandbox
{
    /// <summary>
    /// レスポンスの badges を BadgeStore に反映する。
    /// </summary>
    public sealed class BadgeApplier : IBadgeApplier
    {
        private readonly BadgeStore store;

        /// <summary>
        /// 反映先を受け取って作る。
        /// </summary>
        public BadgeApplier(BadgeStore store)
        {
            this.store = store;
        }

        /// <summary>
        /// レスポンスに badges があれば、入っている種類だけを置き換える。入っていない種類は「変化なし」なので触らない。
        /// </summary>
        public UniTask ApplyAsync(IMessage response)
        {
            // badges を持つレスポンスには、protoc-gen-voltech-grpc が IHasBadges を付けている
            if (response is IHasBadges hasBadges)
            {
                foreach (var badge in hasBadges.Badges)
                {
                    // 古いアプリは、サーバーが後から足した種類を知らないので無視する
                    if (Enum.IsDefined(typeof(BadgeType), badge.Type) && badge.Type != BadgeType.Unspecified)
                    {
                        store.Set(badge);
                    }
                }
            }

            return UniTask.CompletedTask;
        }
    }
}
