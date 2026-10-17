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
            core = new ApiCore(
                new ApiCoreOptions(),
                new CountingTokenProvider(),
                new ErrorPolicyProvider(SandboxErrors.All),
                errorHandler,
                navigator: navigator,
                activityHandlers: new[] { first, second });
            sandboxApi = new SandboxApi(core, invoker);
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
            Assert.That(events, Is.EqualTo(new[] { "first started", "second started" }));

            response.TrySetResult(new PingResponse());
            var result = await pending;

            Assert.That(result.IsSuccess, Is.True);
            Assert.That(events, Is.EqualTo(new[] { "first started", "second started", "second ended", "first ended" }));
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
            Assert.That(events, Is.EqualTo(new[] { "first started", "second started" }), "まだ進行中の呼び出しがあるのに終了が届いた");

            secondResponse.TrySetResult(new PingResponse());
            await secondCall;
            Assert.That(events, Is.EqualTo(new[] { "first started", "second started", "second ended", "first ended" }));
        });

        /// <summary>
        /// エラーハンドラーの間と、再試行で送り直す間も進行中のままで、終了・開始は届かない。
        /// </summary>
        [UnityTest]
        [Description("エラーハンドラーの間と、再試行で送り直す間も進行中のままで、終了・開始は届かない")]
        public IEnumerator ErrorHandlerAndRetryStayActive() => UniTask.ToCoroutine(async () =>
        {
            invoker.Register<PingRequest, PingResponse>(SandboxServiceMethods.Ping, (_, _) =>
                invoker.Calls.Count == 1
                    ? throw FakeRpcErrors.Status(StatusCode.Unavailable)
                    : UniTask.FromResult(new PingResponse()));
            var eventsInHandler = default(string[]);
            errorHandler.OnHandle = () =>
            {
                eventsInHandler = events.ToArray();
                return UniTask.CompletedTask;
            };
            errorHandler.ThenReturn(ErrorDecision.Retry);

            var result = await sandboxApi.PingAsync(new PingRequest());

            Assert.That(result.IsSuccess, Is.True);
            Assert.That(eventsInHandler, Is.EqualTo(new[] { "first started", "second started" }));
            Assert.That(events, Is.EqualTo(new[] { "first started", "second started", "second ended", "first ended" }));
        });

        /// <summary>
        /// notifyActivity を false にした呼び出しは、開始・終了を知らせない。
        /// </summary>
        [UnityTest]
        [Description("notifyActivity を false にした呼び出しは、開始・終了を知らせない")]
        public IEnumerator CallWithoutNotifyActivityIsNotNotified() => UniTask.ToCoroutine(async () =>
        {
            invoker.Register<PingRequest, PingResponse>(SandboxServiceMethods.Ping, (_, _) => UniTask.FromResult(new PingResponse()));

            var result = await sandboxApi.PingAsync(new PingRequest(), notifyActivity: false);

            Assert.That(result.IsSuccess, Is.True);
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
            Assert.That(events, Is.EqualTo(new[] { "first started", "second started", "second ended", "first ended" }));
        });

        /// <summary>
        /// エラーハンドラーがタイトルに戻ると決めたら、遷移より先に終了が届く。
        /// </summary>
        [UnityTest]
        [Description("エラーハンドラーがタイトルに戻ると決めたら、遷移より先に終了が届く")]
        public IEnumerator EndsBeforeNavigation() => UniTask.ToCoroutine(async () =>
        {
            invoker.Register<PingRequest, PingResponse>(SandboxServiceMethods.Ping, (_, _) =>
                throw FakeRpcErrors.Status(StatusCode.Internal));
            errorHandler.ThenReturn(ErrorDecision.ReturnToTitle);
            var eventsAtNavigation = default(string[]);
            navigator.OnGoToTitle = () =>
            {
                eventsAtNavigation = events.ToArray();
                return UniTask.CompletedTask;
            };

            await sandboxApi.PingAsync(new PingRequest());

            Assert.That(navigator.Navigated, Is.EqualTo(new[] { "title" }));
            Assert.That(eventsAtNavigation, Is.EqualTo(new[] { "first started", "second started", "second ended", "first ended" }));
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
            Assert.That(events, Is.EqualTo(new[] { "first started", "second started", "second ended", "first ended" }));
        });
    }
}
