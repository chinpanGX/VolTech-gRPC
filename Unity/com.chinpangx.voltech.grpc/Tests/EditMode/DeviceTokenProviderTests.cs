using System;
using System.Collections;
using System.Linq;
using VolTech.Client.Auth;
using Cysharp.Threading.Tasks;
using Google.Protobuf.WellKnownTypes;
using NUnit.Framework;
using UnityEngine.TestTools;
using VolTech.Client;
using VolTech.V1;

namespace VolTech.Client.Tests.EditMode
{
    /// <summary>
    /// DeviceTokenProvider の、デバイス登録とトークンの発行の自動化を、サーバーなしで確かめる。
    /// </summary>
    [Description("DeviceTokenProvider の、デバイス登録とトークンの発行の自動化を、サーバーなしで確かめる")]
    public class DeviceTokenProviderTests
    {
        private const string RegisterMethod = "/voltech.v1.DeviceService/Register";
        private const string IssueTokenMethod = "/voltech.v1.DeviceService/IssueToken";

        private static readonly DateTime now = new(2026, 10, 4, 12, 0, 0, DateTimeKind.Utc);

        private FakeCallInvoker invoker;
        private InMemoryAccountStore accountStore;
        private DateTime currentTime;
        private DeviceTokenProvider provider;

        /// <summary>
        /// テストごとに、トークンの有効期限が1時間の偽の DeviceService を作る。
        /// </summary>
        [SetUp]
        public void SetUp()
        {
            currentTime = now;
            invoker = new FakeCallInvoker()
                .Register<RegisterRequest, RegisterResponse>(RegisterMethod, async (_, _) =>
                {
                    // 同時に呼ばれたら重なるように、少し待ってから返す
                    await UniTask.Yield();
                    return new RegisterResponse { DeviceId = "device-1", PlayerId = "player-1" };
                })
                .Register<IssueTokenRequest, IssueTokenResponse>(IssueTokenMethod, async (_, _) =>
                {
                    await UniTask.Yield();
                    var issuedCount = invoker.Calls.Count(call => call.MethodFullName == IssueTokenMethod);
                    return new IssueTokenResponse
                    {
                        AccessToken = $"token-{issuedCount}",
                        ExpiresAt = Timestamp.FromDateTime(currentTime.AddHours(1)),
                    };
                });
            accountStore = new InMemoryAccountStore();
            provider = new DeviceTokenProvider(invoker, accountStore, new ApiCoreOptions(), () => currentTime);
        }

        /// <summary>
        /// 未登録なら、デバイス登録してからトークンを発行し、登録した device_id・secret_key・player_id を保存する。
        /// </summary>
        [UnityTest]
        [Description("未登録なら、デバイス登録してからトークンを発行し、登録した device_id・secret_key・player_id を保存する")]
        public IEnumerator RegistersDeviceWhenNotRegistered() => UniTask.ToCoroutine(async () =>
        {
            var token = await provider.GetTokenAsync();

            Assert.That(token, Is.EqualTo("token-1"));
            var saved = await accountStore.LoadAsync();
            Assert.That(saved.DeviceId, Is.EqualTo("device-1"));
            Assert.That(saved.PlayerId, Is.EqualTo("player-1"));
            var registered = (RegisterRequest)invoker.Calls.First().Request;
            Assert.That(saved.SecretKey, Is.EqualTo(registered.SecretKey));
            var issued = (IssueTokenRequest)invoker.Calls.Last().Request;
            Assert.That(issued.SecretKey, Is.EqualTo(registered.SecretKey));
        });

        /// <summary>
        /// デバイス登録とトークンの発行にも、プラットフォームとバージョンを付ける(タイトル画面で強制アップデートを判定してもらうため)。
        /// </summary>
        [UnityTest]
        [Description("デバイス登録とトークンの発行にも、プラットフォームとバージョンを付ける(タイトル画面で強制アップデートを判定してもらうため)")]
        public IEnumerator SendsAppVersion() => UniTask.ToCoroutine(async () =>
        {
            var options = new ApiCoreOptions(new AppVersion(AppVersion.Android, 42));
            provider = new DeviceTokenProvider(invoker, accountStore, options, () => currentTime);

            await provider.GetTokenAsync();

            Assert.That(invoker.Calls.Select(call => call.MethodFullName), Is.EqualTo(new[] { RegisterMethod, IssueTokenMethod }));
            foreach (var call in invoker.Calls)
            {
                Assert.That(call.GetHeader(ApiHeaders.AppPlatform), Is.EqualTo("android"), call.MethodFullName);
                Assert.That(call.GetHeader(ApiHeaders.AppVersion), Is.EqualTo("42"), call.MethodFullName);
            }
        });

        /// <summary>
        /// 同時に何回呼ばれても、デバイス登録とトークンの発行は1回ずつで、同じトークンが返る。
        /// </summary>
        [UnityTest]
        [Description("同時に何回呼ばれても、デバイス登録とトークンの発行は1回ずつで、同じトークンが返る")]
        public IEnumerator ConcurrentCallsRegisterAndIssueOnce() => UniTask.ToCoroutine(async () =>
        {
            var tokens = await UniTask.WhenAll(new[]
            {
                provider.GetTokenAsync(),
                provider.GetTokenAsync(),
                provider.GetTokenAsync(),
            });

            Assert.That(tokens, Is.All.EqualTo("token-1"));
            Assert.That(invoker.Calls.Count(call => call.MethodFullName == RegisterMethod), Is.EqualTo(1));
            Assert.That(invoker.Calls.Count(call => call.MethodFullName == IssueTokenMethod), Is.EqualTo(1));
        });

        /// <summary>
        /// 保存済みのアカウントがあれば、デバイス登録はせずにトークンだけ発行する。
        /// </summary>
        [UnityTest]
        [Description("保存済みのアカウントがあれば、デバイス登録はせずにトークンだけ発行する")]
        public IEnumerator SavedAccountSkipsRegister() => UniTask.ToCoroutine(async () =>
        {
            await accountStore.SaveAsync(new AccountData("saved-device", "saved-secret-key-0000", "saved-player"));

            await provider.GetTokenAsync();

            var issued = (IssueTokenRequest)invoker.Calls.Single().Request;
            Assert.That(issued.DeviceId, Is.EqualTo("saved-device"));
        });

        /// <summary>
        /// 有効期限まで5分を切るまでは同じトークンを返し、切ったら発行し直す。
        /// </summary>
        [UnityTest]
        [Description("有効期限まで5分を切るまでは同じトークンを返し、切ったら発行し直す")]
        public IEnumerator ReissuesWhenNearExpiry() => UniTask.ToCoroutine(async () =>
        {
            var first = await provider.GetTokenAsync();

            currentTime = now.AddMinutes(54);
            Assert.That(await provider.GetTokenAsync(), Is.EqualTo(first));

            currentTime = now.AddMinutes(56);
            Assert.That(await provider.GetTokenAsync(), Is.EqualTo("token-2"));
        });

        /// <summary>
        /// サーバーに弾かれたトークンを Invalidate すると、次は発行し直す。古いトークンを Invalidate しても今のトークンは捨てない。
        /// </summary>
        [UnityTest]
        [Description("サーバーに弾かれたトークンを Invalidate すると、次は発行し直す。古いトークンを Invalidate しても今のトークンは捨てない")]
        public IEnumerator InvalidateReissuesOnlyCurrentToken() => UniTask.ToCoroutine(async () =>
        {
            var first = await provider.GetTokenAsync();
            provider.Invalidate(first);
            var second = await provider.GetTokenAsync();

            provider.Invalidate(first);

            Assert.That(second, Is.EqualTo("token-2"));
            Assert.That(await provider.GetTokenAsync(), Is.EqualTo(second));
        });
    }
}
