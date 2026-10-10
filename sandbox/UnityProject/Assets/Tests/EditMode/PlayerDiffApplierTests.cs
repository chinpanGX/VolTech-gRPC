using System.Collections;
using System.Collections.Generic;
using Cysharp.Threading.Tasks;
using NUnit.Framework;
using Sandbox.V1;
using UnityEngine.TestTools;

namespace Sandbox.Tests.EditMode
{
    /// <summary>
    /// レスポンスの player_diff が PlayerRepository に反映されることを、サーバーなしで確かめる。
    /// </summary>
    [Description("レスポンスの player_diff が PlayerRepository に反映されることを、サーバーなしで確かめる")]
    public class PlayerDiffApplierTests
    {
        private PlayerRepository repository;
        private PlayerDiffApplier applier;

        /// <summary>
        /// テストごとに、ジェム 1000・アイテム1が3個・アイテム2が1個の Repository を作る。
        /// </summary>
        [SetUp]
        public void SetUp()
        {
            repository = new PlayerRepository();
            repository.SetGems(1000);
            repository.UpsertItems(new[]
            {
                new PlayerItem { ItemId = 1, Count = 3 },
                new PlayerItem { ItemId = 2, Count = 1 },
            });
            applier = new PlayerDiffApplier(repository);
        }

        /// <summary>
        /// player_diff のジェムと、追加・更新・削除されたアイテムが反映され、変わっていないものはそのまま残る。
        /// </summary>
        [UnityTest]
        [Description("player_diff のジェムと、追加・更新・削除されたアイテムが反映され、変わっていないものはそのまま残る")]
        public IEnumerator AppliesGemsAndItems() => UniTask.ToCoroutine(async () =>
        {
            var response = new DrawResponse
            {
                PlayerDiff = new PlayerDiff
                {
                    Gems = 700,
                    Items = new ItemsDiff
                    {
                        Upserted = { new PlayerItem { ItemId = 3, Count = 1 } },
                        Removed = { 2 },
                    },
                },
            };

            await applier.ApplyAsync(response);

            Assert.That(repository.Gems, Is.EqualTo(700));
            Assert.That(repository.Items, Is.EquivalentTo(new Dictionary<int, long> { [1] = 3, [3] = 1 }));
        });

        /// <summary>
        /// 同じ player_diff を2回反映しても、1回反映したときと同じになる(値は変更後の値なので)。
        /// </summary>
        [UnityTest]
        [Description("同じ player_diff を2回反映しても、1回反映したときと同じになる(値は変更後の値なので)")]
        public IEnumerator ApplyingTwiceIsSameAsOnce() => UniTask.ToCoroutine(async () =>
        {
            var response = new DrawResponse
            {
                PlayerDiff = new PlayerDiff
                {
                    Gems = 700,
                    Items = new ItemsDiff { Upserted = { new PlayerItem { ItemId = 1, Count = 4 } } },
                },
            };

            await applier.ApplyAsync(response);
            await applier.ApplyAsync(response);

            Assert.That(repository.Gems, Is.EqualTo(700));
            Assert.That(repository.Items[1], Is.EqualTo(4));
        });

        /// <summary>
        /// player_diff が無いレスポンス・ジェムやアイテムの変更が無い player_diff では、何も変わらない。
        /// </summary>
        [UnityTest]
        [Description("player_diff が無いレスポンス・ジェムやアイテムの変更が無い player_diff では、何も変わらない")]
        public IEnumerator NothingChangesWithoutDiff() => UniTask.ToCoroutine(async () =>
        {
            await applier.ApplyAsync(new SignUpResponse { Player = new Player { PlayerId = "player-1", Gems = 1 } });
            await applier.ApplyAsync(new DrawResponse());
            await applier.ApplyAsync(new DrawResponse { PlayerDiff = new PlayerDiff() });

            Assert.That(repository.Gems, Is.EqualTo(1000));
            Assert.That(repository.Items, Is.EquivalentTo(new Dictionary<int, long> { [1] = 3, [2] = 1 }));
        });
    }
}
