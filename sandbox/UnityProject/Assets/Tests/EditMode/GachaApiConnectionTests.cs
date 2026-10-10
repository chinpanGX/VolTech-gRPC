using System.Collections;
using System.Linq;
using Cysharp.Threading.Tasks;
using NUnit.Framework;
using Sandbox.V1;
using UnityEngine.TestTools;
using VolTech.Client;

namespace Sandbox.Tests.EditMode
{
    /// <summary>
    /// 起動しているサーバー(make sandbox-server)に、ApiCore と GachaApi を通してガチャを引く。
    /// </summary>
    [Description("起動しているサーバー(make sandbox-server)に、ApiCore と GachaApi を通してガチャを引く。サーバーを起動してから実行する")]
    public class GachaApiConnectionTests : ApiServerTestBase
    {
        // サンプルの初期ジェム(3000)と1回の消費(300)で引ける回数より多くしておく
        private const int MaxDraws = 20;

        private PlayerApi playerApi;
        private GachaApi gachaApi;

        /// <summary>
        /// テストごとに PlayerApi・GachaApi を作る。
        /// </summary>
        [SetUp]
        public void CreateApis()
        {
            playerApi = new PlayerApi(Core, CallInvoker);
            gachaApi = new GachaApi(Core, CallInvoker);
        }

        /// <summary>
        /// ガチャを引くと、返ってきた時点で player_diff が Repository に反映済みで、サインインで取り直した全件と一致する。
        /// </summary>
        [UnityTest]
        [Description("ガチャを引くと、返ってきた時点で player_diff が Repository に反映済みで、サインインで取り直した全件と一致する")]
        public IEnumerator DrawAppliesPlayerDiffToRepository() => UniTask.ToCoroutine(async () =>
        {
            await SignUpAsync();
            var signedIn = await playerApi.SignInAsync(new SignInRequest());
            // サインインの全件で置き換える(全件に無いものを残さないように、先に消す)
            Repository.SetGems(signedIn.Value.Player.Gems);
            Repository.ClearItems();
            Repository.UpsertItems(signedIn.Value.Items);
            var gemsBefore = Repository.Gems;

            var drawn = await gachaApi.DrawAsync(new DrawRequest());

            Assert.That(drawn.IsSuccess, Is.True, drawn.Error?.ToString());
            Assert.That(drawn.Value.Item.Count, Is.EqualTo(1));
            Assert.That(Repository.Gems, Is.LessThan(gemsBefore));
            Assert.That(Repository.Items[drawn.Value.Item.ItemId], Is.EqualTo(1));

            var reloaded = await playerApi.SignInAsync(new SignInRequest());
            Assert.That(Repository.Gems, Is.EqualTo(reloaded.Value.Player.Gems));
            Assert.That(Repository.Items, Is.EquivalentTo(reloaded.Value.Items.ToDictionary(item => item.ItemId, item => item.Count)));
        });

        /// <summary>
        /// ジェムが尽きるまで引くと、最後は INSUFFICIENT_GEMS(action は Stay)がエラーハンドラーに渡り、ジェムは減らない。
        /// </summary>
        [UnityTest]
        [Description("ジェムが尽きるまで引くと、最後は INSUFFICIENT_GEMS(action は Stay)がエラーハンドラーに渡り、ジェムは減らない")]
        public IEnumerator DrawingUntilGemsRunOutIsInsufficientGems() => UniTask.ToCoroutine(async () =>
        {
            await SignUpAsync();
            ApiResult<DrawResponse> last = default;
            var draws = 0;
            for (; draws < MaxDraws; draws++)
            {
                last = await gachaApi.DrawAsync(new DrawRequest());
                if (!last.IsSuccess)
                {
                    break;
                }
            }

            Assert.That(last.IsSuccess, Is.False, $"{MaxDraws} 回引いてもジェムが尽きない");
            Assert.That(last.Error.Code, Is.EqualTo(SandboxErrors.InsufficientGems));
            var handled = ErrorHandler.Errors.Single();
            Assert.That(handled.Code, Is.EqualTo(SandboxErrors.InsufficientGems));
            Assert.That(handled.Action, Is.EqualTo(ErrorAction.Stay));

            var signedIn = await playerApi.SignInAsync(new SignInRequest());
            Assert.That(signedIn.Value.Items.Sum(item => item.Count), Is.EqualTo(draws));
        });

        /// <summary>
        /// サインインで無料ガチャのバッジが表示になり、無料ガチャを引くと非表示になる。同じ日の2回目は FREE_DRAW_USED(Stay)。
        /// </summary>
        [UnityTest]
        [Description("サインインで無料ガチャのバッジが表示になり、無料ガチャを引くと非表示になる。同じ日の2回目は FREE_DRAW_USED(Stay)")]
        public IEnumerator FreeDrawUpdatesBadgeAndOnlyOncePerDay() => UniTask.ToCoroutine(async () =>
        {
            await SignUpAsync();
            var signedIn = await playerApi.SignInAsync(new SignInRequest());
            Assert.That(signedIn.IsSuccess, Is.True, signedIn.Error?.ToString());
            Assert.That(Badges.IsVisible(BadgeType.FreeGacha), Is.True);

            var free = await gachaApi.DrawAsync(new DrawRequest { Free = true });
            Assert.That(free.IsSuccess, Is.True, free.Error?.ToString());
            Assert.That(free.Value.PlayerDiff.HasGems, Is.False, "無料ガチャはジェムを使わない");
            Assert.That(Badges.IsVisible(BadgeType.FreeGacha), Is.False);

            var again = await gachaApi.DrawAsync(new DrawRequest { Free = true });
            Assert.That(again.IsSuccess, Is.False);
            Assert.That(again.Error.Code, Is.EqualTo(SandboxErrors.FreeDrawUsed));
            Assert.That(again.Error.Action, Is.EqualTo(ErrorAction.Stay));
            Assert.That(ErrorHandler.Errors.Single().Code, Is.EqualTo(SandboxErrors.FreeDrawUsed));
        });

        private async UniTask SignUpAsync()
        {
            var signedUp = await playerApi.SignUpAsync(new SignUpRequest());
            Assert.That(signedUp.IsSuccess, Is.True, signedUp.Error?.ToString());
        }
    }
}
