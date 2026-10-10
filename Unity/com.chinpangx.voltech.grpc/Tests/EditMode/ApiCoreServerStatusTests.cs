using System;
using System.Collections;
using System.Collections.Generic;
using System.Linq;
using Api.Errors;
using App.V1;
using Cysharp.Threading.Tasks;
using NUnit.Framework;
using Tests.EditMode.TestDoubles;
using UnityEngine.TestTools;
using VolTech.Grpc.Client;

namespace Tests.EditMode
{
    /// <summary>
    /// ApiCore の、サーバーの状態の判定に使うヘッダー(アプリのバージョン・ゲーム内の日付)と、サーバー時刻の同期を、サーバーなしで確かめる。
    /// </summary>
    [Description("ApiCore の、サーバーの状態の判定に使うヘッダー(アプリのバージョン・ゲーム内の日付)と、サーバー時刻の同期を、サーバーなしで確かめる")]
    public class ApiCoreServerStatusTests
    {
        private static readonly DateTimeOffset serverTime = new(2026, 10, 6, 3, 0, 0, TimeSpan.Zero);

        private FakeCallInvoker invoker;
        private RecordingErrorHandler errorHandler;
        private TimeSpan elapsed;
        private ApiCore core;
        private PlayerApi playerApi;

        /// <summary>
        /// テストごとに、Windows のバージョン 120 の ApiCore と、成功を返す偽の SignIn を作る。
        /// </summary>
        [SetUp]
        public void SetUp()
        {
            invoker = new FakeCallInvoker();
            invoker.Register<SignInRequest, SignInResponse>(PlayerServiceMethods.SignIn, (_, _) =>
                UniTask.FromResult(new SignInResponse { Player = new Player { PlayerId = "player-1" } }));
            errorHandler = new RecordingErrorHandler();
            elapsed = TimeSpan.Zero;
            core = new ApiCore(
                new ApiCoreOptions(new AppVersion(AppVersion.Windows, 120)),
                new CountingTokenProvider(),
                ProtoErrorPolicyProvider.ForAppErrorCodes(),
                errorHandler,
                serverClock: new ServerClock(() => elapsed));
            playerApi = new PlayerApi(core, invoker);
        }

        /// <summary>
        /// すべてのリクエストに、プラットフォームとバージョンを付ける。
        /// </summary>
        [UnityTest]
        [Description("すべてのリクエストに、プラットフォームとバージョンを付ける")]
        public IEnumerator SendsAppVersion() => UniTask.ToCoroutine(async () =>
        {
            await playerApi.SignInAsync(new SignInRequest());

            var call = invoker.Calls.Single();
            Assert.That(call.GetHeader(ApiHeaders.AppPlatform), Is.EqualTo("windows"));
            Assert.That(call.GetHeader(ApiHeaders.AppVersion), Is.EqualTo("120"));
        });

        /// <summary>
        /// セッションの最初のリクエストには日付を付けず、最初に受け取った日付を以降のリクエストに付け続ける(後から届いた日付には変えない)。
        /// </summary>
        [UnityTest]
        [Description("セッションの最初のリクエストには日付を付けず、最初に受け取った日付を以降のリクエストに付け続ける(後から届いた日付には変えない)")]
        public IEnumerator KeepsFirstGameDateOfSession() => UniTask.ToCoroutine(async () =>
        {
            invoker.ResponseHeaders.Add(ApiHeaders.GameDate, "2026-10-06");
            await playerApi.SignInAsync(new SignInRequest());

            invoker.ResponseHeaders.Clear();
            invoker.ResponseHeaders.Add(ApiHeaders.GameDate, "2026-10-07");
            await playerApi.SignInAsync(new SignInRequest());
            await playerApi.SignInAsync(new SignInRequest());

            Assert.That(invoker.Calls.Select(call => call.GetHeader(ApiHeaders.GameDate)),
                Is.EqualTo(new[] { null, "2026-10-06", "2026-10-06" }));
        });

        /// <summary>
        /// セッションを破棄すると覚えていた日付も捨て、次のリクエストには日付を付けない。
        /// </summary>
        [UnityTest]
        [Description("セッションを破棄すると覚えていた日付も捨て、次のリクエストには日付を付けない")]
        public IEnumerator ResetSessionForgetsGameDate() => UniTask.ToCoroutine(async () =>
        {
            invoker.ResponseHeaders.Add(ApiHeaders.GameDate, "2026-10-06");
            await playerApi.SignInAsync(new SignInRequest());

            core.ResetSession();
            await playerApi.SignInAsync(new SignInRequest());

            Assert.That(invoker.Calls.Last().GetHeader(ApiHeaders.GameDate), Is.Null);
        });

        /// <summary>
        /// DATE_CHANGED でタイトルに戻ると、覚えていた日付を捨てる(サインインし直しで新しい日付を受け取れる)。
        /// </summary>
        [UnityTest]
        [Description("DATE_CHANGED でタイトルに戻ると、覚えていた日付を捨てる(サインインし直しで新しい日付を受け取れる)")]
        public IEnumerator DateChangedReturnToTitleForgetsGameDate() => UniTask.ToCoroutine(async () =>
        {
            invoker.ResponseHeaders.Add(ApiHeaders.GameDate, "2026-10-06");
            await playerApi.SignInAsync(new SignInRequest());

            invoker.Register<SignInRequest, SignInResponse>(PlayerServiceMethods.SignIn, (_, _) =>
                throw FakeRpcErrors.FailedPrecondition("ERROR_CODE_DATE_CHANGED"));
            errorHandler.ThenReturn(ErrorDecision.ReturnToTitle);
            var changed = await playerApi.SignInAsync(new SignInRequest());
            Assert.That(changed.Error.Code, Is.EqualTo((int)ErrorCode.DateChanged));
            Assert.That(changed.Error.Action, Is.EqualTo(ErrorAction.Title));

            await playerApi.SignInAsync(new SignInRequest());
            Assert.That(invoker.Calls.Last().GetHeader(ApiHeaders.GameDate), Is.Null);
        });

        /// <summary>
        /// 成功したレスポンスのサーバー時刻で ServerClock を合わせ、その後の経過時間を足した時刻を返す。
        /// </summary>
        [UnityTest]
        [Description("成功したレスポンスのサーバー時刻で ServerClock を合わせ、その後の経過時間を足した時刻を返す")]
        public IEnumerator SynchronizesServerClock() => UniTask.ToCoroutine(async () =>
        {
            Assert.That(core.ServerClock.IsSynchronized, Is.False);

            invoker.ResponseHeaders.Add(ApiHeaders.ServerTime, serverTime.ToUnixTimeMilliseconds().ToString());
            await playerApi.SignInAsync(new SignInRequest());
            elapsed = TimeSpan.FromSeconds(5);

            Assert.That(core.ServerClock.IsSynchronized, Is.True);
            Assert.That(core.ServerClock.Now, Is.EqualTo(serverTime.AddSeconds(5)));
        });

        /// <summary>
        /// MAINTENANCE の終了予定(ErrorInfo.metadata の end_at)を ApiError.Metadata で読める。metadata が無いエラーでは空。
        /// </summary>
        [UnityTest]
        [Description("MAINTENANCE の終了予定(ErrorInfo.metadata の end_at)を ApiError.Metadata で読める。metadata が無いエラーでは空")]
        public IEnumerator ErrorMetadataIsReadable() => UniTask.ToCoroutine(async () =>
        {
            invoker.Register<SignInRequest, SignInResponse>(PlayerServiceMethods.SignIn, (_, _) =>
                throw FakeRpcErrors.FailedPrecondition(
                    "ERROR_CODE_MAINTENANCE",
                    metadata: new Dictionary<string, string> { ["end_at"] = "2026-10-06T06:00:00Z" }));
            var maintenance = await playerApi.SignInAsync(new SignInRequest());

            invoker.Register<SignInRequest, SignInResponse>(PlayerServiceMethods.SignIn, (_, _) =>
                throw FakeRpcErrors.FailedPrecondition("ERROR_CODE_UPDATE_REQUIRED"));
            var updateRequired = await playerApi.SignInAsync(new SignInRequest());

            Assert.That(maintenance.Error.Metadata["end_at"], Is.EqualTo("2026-10-06T06:00:00Z"));
            Assert.That(updateRequired.Error.Metadata, Is.Empty);
        });
    }
}
