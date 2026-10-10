using System.Collections;
using Cysharp.Threading.Tasks;
using NUnit.Framework;
using Sandbox.V1;
using UnityEngine.TestTools;

namespace Sandbox.Tests.EditMode
{
    /// <summary>
    /// レスポンスの badges が BadgeStore に反映されることを、サーバーなしで確かめる。
    /// </summary>
    [Description("レスポンスの badges が BadgeStore に反映されることを、サーバーなしで確かめる")]
    public class BadgeApplierTests
    {
        /// <summary>
        /// 入っている種類だけを置き換え、入っていない種類と知らない種類は触らない。
        /// </summary>
        [UnityTest]
        [Description("入っている種類だけを置き換え、入っていない種類と知らない種類は触らない")]
        public IEnumerator ReplacesOnlyIncludedTypes() => UniTask.ToCoroutine(async () =>
        {
            var store = new BadgeStore();
            var applier = new BadgeApplier(store);

            await applier.ApplyAsync(new SignInResponse { Badges = { Badge(BadgeType.FreeGacha, true) } });
            Assert.That(store.IsVisible(BadgeType.FreeGacha), Is.True);

            // バッジが入っていないレスポンスでは変わらない
            await applier.ApplyAsync(new DrawResponse());
            Assert.That(store.IsVisible(BadgeType.FreeGacha), Is.True);

            // サーバーが後から足した、このアプリが知らない種類は無視する
            await applier.ApplyAsync(new DrawResponse { Badges = { Badge((BadgeType)999, true) } });
            Assert.That(store.IsVisible((BadgeType)999), Is.False);

            await applier.ApplyAsync(new DrawResponse { Badges = { Badge(BadgeType.FreeGacha, false) } });
            Assert.That(store.IsVisible(BadgeType.FreeGacha), Is.False);
        });

        private static Badge Badge(BadgeType type, bool visible)
        {
            return new Badge { Type = type, Visible = visible };
        }
    }
}
