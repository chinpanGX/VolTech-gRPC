using System;
using System.Collections;
using Cysharp.Threading.Tasks;
using NUnit.Framework;
using Sandbox.V1;
using UnityEngine.TestTools;

namespace Sandbox.Tests.EditMode
{
    /// <summary>
    /// 起動しているサーバー(make sandbox-server)に、ApiCore と PlayerApi を通して、起動時の流れ(デバイス登録 → サインイン → サインアップ)を通す。
    /// </summary>
    [Description("起動しているサーバー(make sandbox-server)に、ApiCore と PlayerApi を通して、起動時の流れ(デバイス登録 → サインイン → サインアップ)を通す。サーバーを起動してから実行する")]
    public class PlayerApiConnectionTests : ApiServerTestBase
    {
        private PlayerApi playerApi;

        /// <summary>
        /// テストごとに PlayerApi を作る。
        /// </summary>
        [SetUp]
        public void CreatePlayerApi()
        {
            playerApi = new PlayerApi(Core, CallInvoker);
        }

        /// <summary>
        /// 新しい端末で最初にサインインすると、デバイス登録とトークンの発行が自動で行われ、PLAYER_NOT_FOUND が返る。
        /// </summary>
        [UnityTest]
        [Description("新しい端末で最初にサインインすると、デバイス登録とトークンの発行が自動で行われ、PLAYER_NOT_FOUND が返る")]
        public IEnumerator FirstSignInRegistersDeviceAndReturnsPlayerNotFound() => UniTask.ToCoroutine(async () =>
        {
            var result = await playerApi.SignInAsync(new SignInRequest(), new[] { SandboxErrors.PlayerNotFound });

            Assert.That(result.IsSuccess, Is.False);
            Assert.That(result.Error.Code, Is.EqualTo(SandboxErrors.PlayerNotFound));
            Assert.That(ErrorHandler.Errors, Is.Empty);
            var saved = await AccountStore.LoadAsync();
            Assert.That(saved.DeviceId, Has.Length.EqualTo(26));
            Assert.That(saved.PlayerId, Has.Length.EqualTo(26), "デバイス登録で player_id が保存されていない");
        });

        /// <summary>
        /// 成功したレスポンスでサーバー時刻とゲーム内の日付を受け取り、日付を付けた次のリクエストもサーバーに通る。
        /// </summary>
        [UnityTest]
        [Description("成功したレスポンスでサーバー時刻とゲーム内の日付を受け取り、日付を付けた次のリクエストもサーバーに通る")]
        public IEnumerator ReceivesServerTimeAndSendsGameDate() => UniTask.ToCoroutine(async () =>
        {
            var signedUp = await playerApi.SignUpAsync(new SignUpRequest());
            Assert.That(signedUp.IsSuccess, Is.True, signedUp.Error?.ToString());
            Assert.That(Core.ServerClock.IsSynchronized, Is.True);
            // サーバーは同じ PC で動いているので、端末の時計とほぼ同じになる
            Assert.That(Core.ServerClock.Now, Is.EqualTo(DateTimeOffset.UtcNow).Within(TimeSpan.FromMinutes(1)));

            // サインアップで受け取った日付が付き、今の日付と同じなので DATE_CHANGED にならない
            var signedIn = await playerApi.SignInAsync(new SignInRequest());
            Assert.That(signedIn.IsSuccess, Is.True, signedIn.Error?.ToString());
            Assert.That(ErrorHandler.Errors, Is.Empty);
        });

        /// <summary>
        /// サインインで PLAYER_NOT_FOUND を受け取ってサインアップすると、次のサインインで同じプレイヤーが返る。
        /// </summary>
        [UnityTest]
        [Description("サインインで PLAYER_NOT_FOUND を受け取ってサインアップすると、次のサインインで同じプレイヤーが返る")]
        public IEnumerator SignUpAfterPlayerNotFoundThenSignIn() => UniTask.ToCoroutine(async () =>
        {
            var notFound = await playerApi.SignInAsync(new SignInRequest(), new[] { SandboxErrors.PlayerNotFound });
            Assert.That(notFound.Error.Code, Is.EqualTo(SandboxErrors.PlayerNotFound));

            var signedUp = await playerApi.SignUpAsync(new SignUpRequest());
            Assert.That(signedUp.IsSuccess, Is.True, signedUp.Error?.ToString());
            // サインアップは、デバイス登録で発行された player_id でプレイヤーを作る
            var saved = await AccountStore.LoadAsync();
            Assert.That(signedUp.Value.Player.PlayerId, Is.EqualTo(saved.PlayerId));

            var signedIn = await playerApi.SignInAsync(new SignInRequest());
            Assert.That(signedIn.IsSuccess, Is.True, signedIn.Error?.ToString());
            Assert.That(signedIn.Value.Player, Is.EqualTo(signedUp.Value.Player));
            Assert.That(ErrorHandler.Errors, Is.Empty);
        });
    }
}
