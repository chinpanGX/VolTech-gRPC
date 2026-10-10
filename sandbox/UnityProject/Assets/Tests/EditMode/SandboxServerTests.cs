using System;
using System.Collections;
using Cysharp.Threading.Tasks;
using Grpc.Core;
using NUnit.Framework;
using Sandbox.V1;
using UnityEngine.TestTools;
using VolTech.Client;

namespace Sandbox.Tests.EditMode
{
    /// <summary>
    /// 起動している sandbox のサーバー(make sandbox-server)に、ApiCore と SandboxApi を通して、基盤の機能(登録・エラー・冪等性)を確かめる。
    /// サインアップは PlayerApi で行う。
    /// </summary>
    [Description("起動している sandbox のサーバー(make sandbox-server)に、ApiCore と SandboxApi を通して、基盤の機能を確かめる。サーバーを起動してから実行する")]
    public class SandboxServerTests : ApiServerTestBase
    {
        private SandboxApi sandboxApi;
        private PlayerApi playerApi;

        /// <summary>
        /// テストごとに SandboxApi と PlayerApi を作る。
        /// </summary>
        [SetUp]
        public void CreateApis()
        {
            sandboxApi = new SandboxApi(Core, CallInvoker);
            playerApi = new PlayerApi(Core, CallInvoker);
        }

        /// <summary>
        /// 新しい端末で Ping すると、デバイス登録とトークンの発行が自動で行われて PLAYER_NOT_FOUND が返り、
        /// サインアップした後の Ping は通って、共通レスポンス項目が反映され、サーバー時刻を受け取る。
        /// </summary>
        [UnityTest]
        [Description("デバイス登録とトークンの発行が自動で行われ、サインアップ後の Ping で共通レスポンス項目が反映され、サーバー時刻を受け取る")]
        public IEnumerator RegisterSignUpAndPing() => UniTask.ToCoroutine(async () =>
        {
            var notFound = await sandboxApi.PingAsync(new PingRequest(), new[] { SandboxErrors.PlayerNotFound });
            Assert.That(notFound.Error.Code, Is.EqualTo(SandboxErrors.PlayerNotFound));
            var saved = await AccountStore.LoadAsync();
            Assert.That(saved.DeviceId, Has.Length.EqualTo(26));
            Assert.That(saved.PlayerId, Has.Length.EqualTo(26), "デバイス登録で player_id が保存されていない");

            var signedUp = await playerApi.SignUpAsync(new SignUpRequest());
            Assert.That(signedUp.IsSuccess, Is.True, signedUp.Error?.ToString());
            Assert.That(signedUp.Value.Player.PlayerId, Is.EqualTo(saved.PlayerId));

            var pinged = await sandboxApi.PingAsync(new PingRequest());
            Assert.That(pinged.IsSuccess, Is.True, pinged.Error?.ToString());
            Assert.That(pinged.Value.PlayerId, Is.EqualTo(saved.PlayerId));
            Assert.That(Repository.Counter, Is.EqualTo(0));
            Assert.That(Badges.IsVisible(BadgeType.Counter), Is.False, "カウンターが 0 なら非表示");
            Assert.That(Notifications.TryDequeue(out var notification), Is.True);
            Assert.That(notification.EventCase, Is.EqualTo(Notification.EventOneofCase.PingReceived));
            Assert.That(Core.ServerClock.IsSynchronized, Is.True);
            // サーバーは同じ PC で動いているので、端末の時計とほぼ同じになる
            Assert.That(Core.ServerClock.Now, Is.EqualTo(DateTimeOffset.UtcNow).Within(TimeSpan.FromMinutes(1)));
            Assert.That(ErrorHandler.Errors, Is.Empty);
        });

        /// <summary>
        /// Fail は指定したエラーコードで失敗し、errors.toml の既定の action が付く。handledCodes に入れなければエラーハンドラーに渡る。
        /// </summary>
        [UnityTest]
        [Description("Fail は指定したエラーコードで失敗し、errors.toml の既定の action が付く")]
        public IEnumerator FailReturnsCodeAndDefaultAction() => UniTask.ToCoroutine(async () =>
        {
            var handled = await sandboxApi.FailAsync(
                Fail(SandboxErrors.SandboxStay),
                new[] { SandboxErrors.SandboxStay });
            Assert.That(handled.IsSuccess, Is.False);
            Assert.That(handled.Error.Code, Is.EqualTo(SandboxErrors.SandboxStay));
            Assert.That(handled.Error.Action, Is.EqualTo(ErrorAction.Stay));
            Assert.That(ErrorHandler.Errors, Is.Empty);

            var maintenance = await sandboxApi.FailAsync(Fail(ReservedErrors.Maintenance));
            Assert.That(maintenance.Error.Code, Is.EqualTo(ReservedErrors.Maintenance));
            Assert.That(maintenance.Error.Action, Is.EqualTo(ErrorAction.Title));
            Assert.That(ErrorHandler.Errors, Has.Count.EqualTo(1));
        });

        private static FailRequest Fail(ErrorCode code)
        {
            return new FailRequest { Domain = code.Domain, Reason = code.Reason };
        }

        /// <summary>
        /// Increment を同じ RequestID で2回送ると、2回目は保存済みのレスポンスが返り、カウンターは1回分だけ増える。
        /// </summary>
        [UnityTest]
        [Description("Increment を同じ RequestID で2回送ると、カウンターは1回分だけ増える")]
        public IEnumerator IncrementWithSameRequestIdIsAppliedOnce() => UniTask.ToCoroutine(async () =>
        {
            var signedUp = await playerApi.SignUpAsync(new SignUpRequest());
            Assert.That(signedUp.IsSuccess, Is.True, signedUp.Error?.ToString());

            // ApiCore は再試行のときだけ同じ RequestID で送るので、ヘッダーを自分で付けて直接送る
            var client = new SandboxService.SandboxServiceClient(CallInvoker);
            var token = await TokenProvider.GetTokenAsync();
            var requestId = Guid.NewGuid().ToString("N");
            var headers = new Metadata
            {
                { ApiHeaders.Authorization, $"Bearer {token}" },
                { ApiHeaders.RequestId, requestId },
            };

            var first = await client.IncrementAsync(new IncrementRequest { Amount = 5 }, headers);
            var resent = await client.IncrementAsync(new IncrementRequest { Amount = 5 }, headers);
            Assert.That(first.Counter, Is.EqualTo(5));
            Assert.That(resent, Is.EqualTo(first));

            // ApiCore を通すと新しい RequestID が付くので、もう1回分増える
            var next = await sandboxApi.IncrementAsync(new IncrementRequest { Amount = 5 });
            Assert.That(next.IsSuccess, Is.True, next.Error?.ToString());
            Assert.That(next.Value.Counter, Is.EqualTo(10));
            Assert.That(Repository.Counter, Is.EqualTo(10));
            Assert.That(Badges.CountOf(BadgeType.Counter), Is.EqualTo(0), "Increment はバッジを返さない");
        });
    }
}
