using System;
using System.Collections;
using System.Collections.Generic;
using Cysharp.Threading.Tasks;
using Grpc.Core;
using NUnit.Framework;
using Sandbox.Tests.EditMode.TestDoubles;
using Sandbox.V1;
using UnityEngine.TestTools;
using VolTech.Client;

namespace Sandbox.Tests.EditMode
{
    /// <summary>
    /// ApiCore が進行中の呼び出しの開始・終了を、アクティビティのハンドラーに正しく知らせるかを、サーバーなしで確かめる。
    /// </summary>
    [Description("ApiCore が進行中の呼び出しの開始・終了を、アクティビティのハンドラーに正しく知らせるかを、サーバーなしで確かめる")]
    public class ApiCoreActivityTests
    {
        // ハンドラー2つに開始が届いたところまでと、終了まで届いたところまで
        private static readonly string[] Started = { "first started", "second started" };
        private static readonly string[] Ended = { "first started", "second started", "second ended", "first ended" };

        private FakeCallInvoker invoker;
        private RecordingErrorHandler errorHandler;
        private RecordingNavigator navigator;
        private List<string> events;
        private RecordingActivityHandler first;
        private RecordingActivityHandler second;
        private ApiCore core;
        private SandboxApi sandboxApi;

        /// <summary>
        /// テストごとに、偽のサーバーと、ハンドラーを2つ登録した ApiCore を作る。
        /// </summary>
        [SetUp]
        public void SetUp()
        {
            invoker = new FakeCallInvoker();
            errorHandler = new RecordingErrorHandler();
            navigator = new RecordingNavigator();
            events = new List<string>();
            first = new RecordingActivityHandler("first", events);
            second = new RecordingActivityHandler("second", events);
            CreateCore(null);
        }

        /// <summary>
        /// 呼び出しの間は開始だけが届き、結果が返る前に終了が届く。開始は登録した順、終了は逆の順に呼ばれる。
        /// </summary>
        [UnityTest]
        [Description("呼び出しの間は開始だけが届き、結果が返る前に終了が届く。開始は登録した順、終了は逆の順に呼ばれる")]
        public IEnumerator CallNotifiesStartAndEndInOrder() => UniTask.ToCoroutine(async () =>
        {
            var response = new UniTaskCompletionSource<PingResponse>();
            invoker.Register<PingRequest, PingResponse>(SandboxServiceMethods.Ping, (_, _) => response.Task);

            var pending = sandboxApi.PingAsync(new PingRequest());
            await UniTask.Yield();
            Assert.That(events, Is.EqualTo(Started));

            response.TrySetResult(new PingResponse());
            var result = await pending;

            Assert.That(result.IsSuccess, Is.True);
            Assert.That(events, Is.EqualTo(Ended));
        });

        /// <summary>
        /// 呼び出しが重なっても、開始は最初の1つ、終了は最後の1つが終わったときにだけ届く。
        /// </summary>
        [UnityTest]
        [Description("呼び出しが重なっても、開始は最初の1つ、終了は最後の1つが終わったときにだけ届く")]
        public IEnumerator OverlappingCallsNotifyOnce() => UniTask.ToCoroutine(async () =>
        {
            var firstResponse = new UniTaskCompletionSource<PingResponse>();
            var secondResponse = new UniTaskCompletionSource<PingResponse>();
            invoker.Register<PingRequest, PingResponse>(SandboxServiceMethods.Ping, (_, _) =>
                invoker.Calls.Count == 1 ? firstResponse.Task : secondResponse.Task);

            var firstCall = sandboxApi.PingAsync(new PingRequest());
            var secondCall = sandboxApi.PingAsync(new PingRequest());
            await UniTask.Yield();

            firstResponse.TrySetResult(new PingResponse());
            await firstCall;
            Assert.That(events, Is.EqualTo(Started), "まだ進行中の呼び出しがあるのに終了が届いた");

            secondResponse.TrySetResult(new PingResponse());
            await secondCall;
            Assert.That(events, Is.EqualTo(Ended));
        });

        /// <summary>
        /// 送信に失敗してエラーハンドラーが Stay を返すと、エラーハンドラーを呼ぶ前に終了が届き、ハンドラーの中では進行中でない。
        /// 終了は1回だけで、次の呼び出しでまた開始から届く。
        /// </summary>
        [UnityTest]
        [Description("送信に失敗してエラーハンドラーが Stay を返すと、エラーハンドラーを呼ぶ前に終了が届き、ハンドラーの中では進行中でない。終了は1回だけで、次の呼び出しでまた開始から届く")]
        public IEnumerator ErrorHandlerRunsAfterEnd() => UniTask.ToCoroutine(async () =>
        {
            invoker.Register<PingRequest, PingResponse>(SandboxServiceMethods.Ping, (_, _) =>
                invoker.Calls.Count == 1
                    ? throw FakeRpcErrors.Status(StatusCode.Unavailable)
                    : UniTask.FromResult(new PingResponse()));
            var eventsInHandler = RecordEventsInHandler();
            errorHandler.ThenReturn(ErrorDecision.Stay);

            var failed = await sandboxApi.PingAsync(new PingRequest());

            Assert.That(failed.IsSuccess, Is.False);
            Assert.That(eventsInHandler, Is.EqualTo(new[] { Ended }));
            Assert.That(events, Is.EqualTo(Ended), "終了が二重に届いた");

            // 開始と終了の数がずれていれば、次の呼び出しで開始・終了が届かない
            events.Clear();
            var next = await sandboxApi.PingAsync(new PingRequest());
            Assert.That(next.IsSuccess, Is.True);
            Assert.That(events, Is.EqualTo(Ended));
        });

        /// <summary>
        /// エラーハンドラーが Retry を返すと、送り直す前にもう一度開始が届く(開始 → 終了 → 開始 → 終了)。
        /// </summary>
        [UnityTest]
        [Description("エラーハンドラーが Retry を返すと、送り直す前にもう一度開始が届く(開始 → 終了 → 開始 → 終了)")]
        public IEnumerator RetryStartsAgainBeforeResend() => UniTask.ToCoroutine(async () =>
        {
            var eventsAtResend = default(string[]);
            invoker.Register<PingRequest, PingResponse>(SandboxServiceMethods.Ping, (_, _) =>
            {
                if (invoker.Calls.Count == 1)
                {
                    throw FakeRpcErrors.Status(StatusCode.Unavailable);
                }

                eventsAtResend = events.ToArray();
                return UniTask.FromResult(new PingResponse());
            });
            var eventsInHandler = RecordEventsInHandler();
            errorHandler.ThenReturn(ErrorDecision.Retry);

            var result = await sandboxApi.PingAsync(new PingRequest());

            Assert.That(result.IsSuccess, Is.True);
            Assert.That(eventsInHandler, Is.EqualTo(new[] { Ended }));
            Assert.That(eventsAtResend, Is.EqualTo(Concat(Ended, Started)));
            Assert.That(events, Is.EqualTo(Concat(Ended, Ended)));
        });

        /// <summary>
        /// エラーハンドラーの間にセッションを破棄していたら、Retry が返っても送り直さず、開始も届かない。
        /// </summary>
        [UnityTest]
        [Description("エラーハンドラーの間にセッションを破棄していたら、Retry が返っても送り直さず、開始も届かない")]
        public IEnumerator RetryAfterResetSessionDoesNotStartAgain() => UniTask.ToCoroutine(async () =>
        {
            invoker.Register<PingRequest, PingResponse>(SandboxServiceMethods.Ping, (_, _) =>
                throw FakeRpcErrors.Status(StatusCode.Unavailable));
            // ダイアログが出ている間に、アカウントの引き継ぎなどでセッションを破棄する
            errorHandler.OnHandle = () =>
            {
                core.ResetSession();
                return UniTask.CompletedTask;
            };
            errorHandler.ThenReturn(ErrorDecision.Retry);

            var result = await sandboxApi.PingAsync(new PingRequest());

            Assert.That(result.Error.IsDiscarded, Is.True);
            Assert.That(invoker.Calls, Has.Count.EqualTo(1), "破棄したセッションで送り直した");
            Assert.That(events, Is.EqualTo(Ended));
        });

        /// <summary>
        /// UNAUTHENTICATED のときのトークンの取り直しと送り直しの間は、進行中のままで、終了・開始は届かない。
        /// </summary>
        [UnityTest]
        [Description("UNAUTHENTICATED のときのトークンの取り直しと送り直しの間は、進行中のままで、終了・開始は届かない")]
        public IEnumerator ReauthenticationStaysActive() => UniTask.ToCoroutine(async () =>
        {
            invoker.Register<PingRequest, PingResponse>(SandboxServiceMethods.Ping, (_, _) =>
                invoker.Calls.Count == 1
                    ? throw FakeRpcErrors.Status(StatusCode.Unauthenticated)
                    : UniTask.FromResult(new PingResponse()));

            var result = await sandboxApi.PingAsync(new PingRequest());

            Assert.That(result.IsSuccess, Is.True);
            Assert.That(invoker.Calls, Has.Count.EqualTo(2));
            Assert.That(events, Is.EqualTo(Ended));
        });

        /// <summary>
        /// 2つの呼び出しが重なっていて、片方がエラーハンドラーにいる間も、もう片方の分で進行中のまま。
        /// もう片方が終わったら終了が届き、エラーハンドラーから抜けても二重には届かない。
        /// </summary>
        [UnityTest]
        [Description("2つの呼び出しが重なっていて、片方がエラーハンドラーにいる間も、もう片方の分で進行中のまま。もう片方が終わったら終了が届き、エラーハンドラーから抜けても二重には届かない")]
        public IEnumerator OtherCallKeepsActiveWhileInErrorHandler() => UniTask.ToCoroutine(async () =>
        {
            var otherResponse = new UniTaskCompletionSource<PingResponse>();
            invoker.Register<PingRequest, PingResponse>(SandboxServiceMethods.Ping, (_, _) =>
                invoker.Calls.Count == 1
                    ? otherResponse.Task
                    : throw FakeRpcErrors.Status(StatusCode.Unavailable));
            var dialog = new UniTaskCompletionSource();
            errorHandler.OnHandle = () => dialog.Task;

            var other = sandboxApi.PingAsync(new PingRequest());
            var failing = sandboxApi.PingAsync(new PingRequest());
            await UniTask.Yield();
            Assert.That(errorHandler.Errors, Has.Count.EqualTo(1));
            Assert.That(events, Is.EqualTo(Started), "ほかに進行中の呼び出しがあるのに終了が届いた");

            otherResponse.TrySetResult(new PingResponse());
            await other;
            Assert.That(events, Is.EqualTo(Ended));

            dialog.TrySetResult();
            await failing;
            Assert.That(events, Is.EqualTo(Ended), "終了が二重に届いた");
        });

        /// <summary>
        /// ほかの呼び出しのエラーハンドラーが終わるのを待っている間は、進行中に数えない。
        /// </summary>
        [UnityTest]
        [Description("ほかの呼び出しのエラーハンドラーが終わるのを待っている間は、進行中に数えない")]
        public IEnumerator WaitingForErrorHandlerIsNotActive() => UniTask.ToCoroutine(async () =>
        {
            // 2つの呼び出しを重ねてから失敗させる
            var firstResponse = new UniTaskCompletionSource<PingResponse>();
            var secondResponse = new UniTaskCompletionSource<PingResponse>();
            invoker.Register<PingRequest, PingResponse>(SandboxServiceMethods.Ping, (_, _) =>
                invoker.Calls.Count == 1 ? firstResponse.Task : secondResponse.Task);
            var dialog = new UniTaskCompletionSource();
            errorHandler.OnHandle = () => dialog.Task;

            var firstCall = sandboxApi.PingAsync(new PingRequest());
            var secondCall = sandboxApi.PingAsync(new PingRequest());
            await UniTask.Yield();
            Assert.That(events, Is.EqualTo(Started));

            firstResponse.TrySetException(FakeRpcErrors.Status(StatusCode.Unavailable));
            secondResponse.TrySetException(FakeRpcErrors.Status(StatusCode.Unavailable));
            await UniTask.Yield();
            Assert.That(errorHandler.Errors, Has.Count.EqualTo(1), "エラーハンドラーが同時に呼ばれた");
            Assert.That(events, Is.EqualTo(Ended), "エラーハンドラーの順番待ちの間も進行中に数えた");

            dialog.TrySetResult();
            await UniTask.WhenAll(firstCall, secondCall);
            Assert.That(errorHandler.Errors, Has.Count.EqualTo(2));
            Assert.That(events, Is.EqualTo(Ended));
        });

        /// <summary>
        /// notifyActivity を false にした呼び出しは、開始・終了を知らせない。エラーハンドラーを呼んでも知らせない。
        /// </summary>
        [UnityTest]
        [Description("notifyActivity を false にした呼び出しは、開始・終了を知らせない。エラーハンドラーを呼んでも知らせない")]
        public IEnumerator CallWithoutNotifyActivityIsNotNotified() => UniTask.ToCoroutine(async () =>
        {
            invoker.Register<PingRequest, PingResponse>(SandboxServiceMethods.Ping, (_, _) =>
                invoker.Calls.Count == 1
                    ? throw FakeRpcErrors.Status(StatusCode.Unavailable)
                    : UniTask.FromResult(new PingResponse()));
            errorHandler.ThenReturn(ErrorDecision.Retry);

            var result = await sandboxApi.PingAsync(new PingRequest(), notifyActivity: false);

            Assert.That(result.IsSuccess, Is.True);
            Assert.That(errorHandler.Errors, Has.Count.EqualTo(1));
            Assert.That(events, Is.Empty);
        });

        /// <summary>
        /// ハンドラーが例外を投げても、残りのハンドラーは呼ばれ、呼び出しの結果にも影響しない。
        /// </summary>
        [UnityTest]
        [Description("ハンドラーが例外を投げても、残りのハンドラーは呼ばれ、呼び出しの結果にも影響しない")]
        public IEnumerator HandlerExceptionDoesNotStopOthers() => UniTask.ToCoroutine(async () =>
        {
            invoker.Register<PingRequest, PingResponse>(SandboxServiceMethods.Ping, (_, _) => UniTask.FromResult(new PingResponse()));
            first.ThrowOnNotify = new InvalidOperationException("bug in handler");

            var result = await sandboxApi.PingAsync(new PingRequest());

            Assert.That(result.IsSuccess, Is.True);
            Assert.That(events, Is.EqualTo(Ended));
        });

        /// <summary>
        /// エラーハンドラーがタイトル・ホームに戻ると決めたら、エラーハンドラーの前に終了が届き、遷移までに開始は届かない。
        /// </summary>
        [UnityTest]
        [Description("エラーハンドラーがタイトル・ホームに戻ると決めたら、エラーハンドラーの前に終了が届き、遷移までに開始は届かない")]
        public IEnumerator EndsBeforeNavigation(
            [Values(ErrorDecision.ReturnToTitle, ErrorDecision.ReturnToHome)] ErrorDecision decision) => UniTask.ToCoroutine(async () =>
        {
            invoker.Register<PingRequest, PingResponse>(SandboxServiceMethods.Ping, (_, _) =>
                throw FakeRpcErrors.Status(StatusCode.Internal));
            var eventsInHandler = RecordEventsInHandler();
            errorHandler.ThenReturn(decision);
            var eventsAtNavigation = RecordEventsAtNavigation();

            await sandboxApi.PingAsync(new PingRequest());

            Assert.That(navigator.Navigated, Has.Count.EqualTo(1));
            Assert.That(eventsInHandler, Is.EqualTo(new[] { Ended }));
            Assert.That(eventsAtNavigation(), Is.EqualTo(Ended));
            Assert.That(events, Is.EqualTo(Ended));
        });

        /// <summary>
        /// 共通レスポンス項目の反映に失敗してエラーハンドラーに回すときも、エラーハンドラーの前に終了が届き、遷移までに開始は届かない。
        /// 反映の失敗では送り直さないので、Retry が返っても開始は届かない。
        /// </summary>
        [UnityTest]
        [Description("共通レスポンス項目の反映に失敗してエラーハンドラーに回すときも、エラーハンドラーの前に終了が届き、遷移までに開始は届かない。反映の失敗では送り直さないので、Retry が返っても開始は届かない")]
        public IEnumerator ApplyFailureEndsBeforeErrorHandler(
            [Values(ErrorDecision.Stay, ErrorDecision.Retry, ErrorDecision.ReturnToTitle, ErrorDecision.ReturnToHome)] ErrorDecision decision) =>
            UniTask.ToCoroutine(async () =>
            {
                CreateCore(new RecordingApplier("player_diff", new List<string>())
                {
                    ThrowOnApply = new InvalidOperationException("bug in applier"),
                });
                invoker.Register<PingRequest, PingResponse>(SandboxServiceMethods.Ping, (_, _) => UniTask.FromResult(new PingResponse()));
                var eventsInHandler = RecordEventsInHandler();
                errorHandler.ThenReturn(decision);
                var eventsAtNavigation = RecordEventsAtNavigation();

                var result = await sandboxApi.PingAsync(new PingRequest());

                Assert.That(result.Error.Code, Is.EqualTo(ReservedErrors.ClientError));
                Assert.That(invoker.Calls, Has.Count.EqualTo(1), "反映の失敗で送り直した");
                Assert.That(eventsInHandler, Is.EqualTo(new[] { Ended }));
                Assert.That(events, Is.EqualTo(Ended));
                if (navigator.Navigated.Count > 0)
                {
                    Assert.That(eventsAtNavigation(), Is.EqualTo(Ended));
                }
            });

        /// <summary>
        /// セッションを捨てて止めた呼び出しも、終了が届く。
        /// </summary>
        [UnityTest]
        [Description("セッションを捨てて止めた呼び出しも、終了が届く")]
        public IEnumerator DiscardedCallEnds() => UniTask.ToCoroutine(async () =>
        {
            var neverResponds = new UniTaskCompletionSource<PingResponse>();
            invoker.Register<PingRequest, PingResponse>(SandboxServiceMethods.Ping, (_, _) => neverResponds.Task);

            var pending = sandboxApi.PingAsync(new PingRequest());
            await UniTask.Yield();
            core.ResetSession();
            var result = await pending;

            Assert.That(result.Error.IsDiscarded, Is.True);
            Assert.That(events, Is.EqualTo(Ended));
        });

        private static string[] Concat(string[] head, string[] tail)
        {
            var all = new List<string>(head);
            all.AddRange(tail);
            return all.ToArray();
        }

        // 反映処理を渡したいテストのために、ApiCore を作り直せるようにする
        private void CreateCore(IPlayerDiffApplier playerDiffApplier)
        {
            core = ApiCore.CreateBuilder(
                    new ApiCoreOptions(),
                    new CountingTokenProvider(),
                    new ErrorPolicyProvider(SandboxErrors.All),
                    errorHandler)
                .WithPlayerDiffApplier(playerDiffApplier)
                .WithErrorNavigator(navigator)
                .AddActivityHandler(first)
                .AddActivityHandler(second)
                .Build();
            sandboxApi = new SandboxApi(core, invoker);
        }

        // エラーハンドラーが呼ばれるたびに、その時点の開始・終了の記録を足す
        private List<string[]> RecordEventsInHandler()
        {
            var recorded = new List<string[]>();
            errorHandler.OnHandle = () =>
            {
                recorded.Add(events.ToArray());
                return UniTask.CompletedTask;
            };
            return recorded;
        }

        // タイトル・ホームに遷移した時点の開始・終了の記録を、後から読めるようにする
        private Func<string[]> RecordEventsAtNavigation()
        {
            var recorded = default(string[]);
            navigator.OnGoToTitle = () =>
            {
                recorded = events.ToArray();
                return UniTask.CompletedTask;
            };
            navigator.OnGoToHome = () =>
            {
                recorded = events.ToArray();
                return UniTask.CompletedTask;
            };
            return () => recorded;
        }
    }
}
