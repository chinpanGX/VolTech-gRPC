using System.Collections.Generic;
using Sandbox.V1;

namespace Sandbox
{
    /// <summary>
    /// サーバーが計算したバッジの今の状態。画面はここを読んでバッジを出す。
    /// </summary>
    /// <remarks>
    /// BadgeApplier が、レスポンスの badges で種類ごとに上書きする。まだ受け取っていない種類は非表示として扱う。
    /// クライアントが持っているデータから判定するバッジ(サーバーから返さないもの)は、ここに入れない。
    /// </remarks>
    public sealed class BadgeStore
    {
        private readonly Dictionary<BadgeType, Badge> badges = new();

        /// <summary>
        /// type のバッジを表示するか。まだ受け取っていなければ false。
        /// </summary>
        public bool IsVisible(BadgeType type)
        {
            return badges.TryGetValue(type, out var badge) && badge.Visible;
        }

        /// <summary>
        /// type のバッジの件数。まだ受け取っていない・件数を出さないバッジは 0。
        /// </summary>
        public int CountOf(BadgeType type)
        {
            return badges.TryGetValue(type, out var badge) ? badge.Count : 0;
        }

        /// <summary>
        /// バッジの状態を、受け取った値で置き換える。
        /// </summary>
        public void Set(Badge badge)
        {
            badges[badge.Type] = badge;
        }

        /// <summary>
        /// 受け取ったバッジをすべて忘れる。タイトルに戻ったときに呼ぶ。
        /// </summary>
        public void Clear()
        {
            badges.Clear();
        }
    }
}
