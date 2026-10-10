using System.Collections.Generic;
using Sandbox.V1;

namespace Sandbox
{
    /// <summary>
    /// クライアントが持つプレイヤーのデータ(ジェム・所持アイテム・カウンター)。
    /// </summary>
    /// <remarks>
    /// API のレスポンスの型は知らず、データの操作だけを持つ。レスポンスの読み解きは呼び出す側で行う。
    /// - サインインの全件: SetGems → SetCounter → ClearItems → UpsertItems(全件に無いものを残さないように、先に消す)
    /// - player_diff: PlayerDiffApplier が SetGems → UpsertItems → RemoveItems → SetCounter
    /// 値はどれも変更後の値なので、同じ値を2回入れても結果は変わらない。
    /// </remarks>
    public sealed class PlayerRepository
    {
        private readonly Dictionary<int, long> items = new();

        /// <summary>持っているジェム。</summary>
        public long Gems { get; private set; }

        /// <summary>持っているアイテムの個数(item_id → 個数)。</summary>
        public IReadOnlyDictionary<int, long> Items => items;

        /// <summary>基盤の冪等性を確かめるためのカウンター(SandboxService/Increment で増える)。</summary>
        public long Counter { get; private set; }

        /// <summary>
        /// ジェムを変更後の値で置き換える。
        /// </summary>
        public void SetGems(long gems)
        {
            Gems = gems;
        }

        /// <summary>
        /// カウンターを変更後の値で置き換える。
        /// </summary>
        public void SetCounter(long counter)
        {
            Counter = counter;
        }

        /// <summary>
        /// アイテムを変更後の個数で入れる。持っていれば上書きし、持っていなければ足す。
        /// </summary>
        public void UpsertItems(IEnumerable<PlayerItem> upserted)
        {
            foreach (var item in upserted)
            {
                items[item.ItemId] = item.Count;
            }
        }

        /// <summary>
        /// 無くなったアイテムを消す。持っていないものは無視する。
        /// </summary>
        public void RemoveItems(IEnumerable<int> itemIds)
        {
            foreach (var itemId in itemIds)
            {
                items.Remove(itemId);
            }
        }

        /// <summary>
        /// 持っているアイテムをすべて消す。
        /// </summary>
        public void ClearItems()
        {
            items.Clear();
        }
    }
}
