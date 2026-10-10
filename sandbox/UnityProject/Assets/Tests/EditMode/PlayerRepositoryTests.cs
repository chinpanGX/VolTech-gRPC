using System.Collections.Generic;
using NUnit.Framework;
using Sandbox.V1;

namespace Sandbox.Tests.EditMode
{
    /// <summary>
    /// PlayerRepository のデータの操作を確かめる。
    /// </summary>
    [Description("PlayerRepository のデータの操作を確かめる")]
    public class PlayerRepositoryTests
    {
        /// <summary>
        /// サインインの全件で置き換えると(ClearItems → UpsertItems)、全件に無いアイテムは残らない。
        /// </summary>
        [Test]
        [Description("サインインの全件で置き換えると(ClearItems → UpsertItems)、全件に無いアイテムは残らない")]
        public void ClearThenUpsertDropsItemsMissingFromSnapshot()
        {
            var repository = new PlayerRepository();
            repository.UpsertItems(new[]
            {
                new PlayerItem { ItemId = 1, Count = 3 },
                new PlayerItem { ItemId = 2, Count = 1 },
            });

            // サーバーではアイテム2が無くなっていて、全件にはアイテム1だけが入っている
            repository.ClearItems();
            repository.UpsertItems(new[] { new PlayerItem { ItemId = 1, Count = 5 } });

            Assert.That(repository.Items, Is.EquivalentTo(new Dictionary<int, long> { [1] = 5 }));
        }

        /// <summary>
        /// 同じアイテムを2回 Upsert しても、後の個数で上書きされるだけで足し算にはならない。
        /// </summary>
        [Test]
        [Description("同じアイテムを2回 Upsert しても、後の個数で上書きされるだけで足し算にはならない")]
        public void UpsertOverwritesWithLatestCount()
        {
            var repository = new PlayerRepository();
            repository.UpsertItems(new[] { new PlayerItem { ItemId = 1, Count = 3 } });
            repository.UpsertItems(new[] { new PlayerItem { ItemId = 1, Count = 3 } });

            Assert.That(repository.Items[1], Is.EqualTo(3));
        }
    }
}
