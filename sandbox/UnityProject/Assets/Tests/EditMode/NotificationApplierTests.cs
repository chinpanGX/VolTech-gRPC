using System.Collections;
using Cysharp.Threading.Tasks;
using NUnit.Framework;
using Sandbox.V1;
using UnityEngine.TestTools;

namespace Sandbox.Tests.EditMode
{
    /// <summary>
    /// レスポンスの notifications が NotificationQueue に積まれることを、サーバーなしで確かめる。
    /// </summary>
    [Description("レスポンスの notifications が NotificationQueue に積まれることを、サーバーなしで確かめる")]
    public class NotificationApplierTests
    {
        /// <summary>
        /// 届いた順に積み、知らない種類(event が空)は積まない。
        /// </summary>
        [UnityTest]
        [Description("届いた順に積み、知らない種類(event が空)は積まない")]
        public IEnumerator QueuesKnownEventsInOrder() => UniTask.ToCoroutine(async () =>
        {
            var queue = new NotificationQueue();
            var applier = new NotificationApplier(queue);

            await applier.ApplyAsync(new DrawResponse
            {
                Notifications =
                {
                    Rare(3),
                    new Notification(),
                    Rare(4),
                },
            });

            Assert.That(queue.Count, Is.EqualTo(2));
            Assert.That(queue.TryDequeue(out var first), Is.True);
            Assert.That(first.RareItemObtained.ItemId, Is.EqualTo(3));
            Assert.That(queue.TryDequeue(out var second), Is.True);
            Assert.That(second.RareItemObtained.ItemId, Is.EqualTo(4));
            Assert.That(queue.TryDequeue(out _), Is.False);
        });

        private static Notification Rare(int itemId)
        {
            return new Notification { RareItemObtained = new RareItemObtained { ItemId = itemId } };
        }
    }
}
