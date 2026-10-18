using System;
using System.Collections;
using System.Collections.Generic;
using System.Linq;
using Cysharp.Threading.Tasks;
using Grpc.Core;
using NUnit.Framework;
using Sandbox;
using Sandbox.Tests.EditMode.TestDoubles;
using Sandbox.V1;
using UnityEngine.TestTools;
using VolTech.Client;
using VolTech.Client.Auth;

namespace Sandbox.Tests.EditMode
{
    /// <summary>
    /// ApiCore の共通処理(トークン・ヘッダー・再認証・エラーの変換とハンドラー・順番待ち)を、サーバーなしで確かめる。
    /// </summary>
    [Description("ApiCore の共通処理(トークン・ヘッダー・再認証・エラーの変換とハンドラー・順番待ち)を、サーバーなしで確かめる")]
    public class ApiCoreTests
    {
        private FakeCallInvoker invoker;
        private CountingTokenProvider tokenProvider;
        private RecordingErrorHandler errorHandler;
        private List<string> applyOrder;
        private RecordingApplier diffApplier;
        private RecordingApplier badgeApplier;
        private RecordingApplier notificationApplier;
        private ApiCore core;
        private SandboxApi sandboxApi;

        /// <summary>
        /// テストごとに、偽のサーバーと ApiCore を作る。
        /// </summary>
        [SetUp]
        public void SetUp()
        {
            invoker = new FakeCallInvoker();
            tokenProvider = new CountingTokenProvider();
            errorHandler = new RecordingErrorHandler();
            applyOrder = new List<string>();
            diffApplier = new RecordingApplier("player_diff", applyOrder);
            badgeApplier = new RecordingApplier("badges", applyOrder);
            notificationApplier = new RecordingApplier("notifications", applyOrder);
            core = ApiCore.CreateBuilder(
                    new ApiCoreOptions(),
                    tokenProvider,
                    new ErrorPolicyProvider(SandboxErrors.All),
                    errorHandler)
                .WithPlayerDiffApplier(diffApplier)
                .WithBadgeApplier(badgeApplier)
                .WithNotificationApplier(notificationApplier)
                .Build();
            sandboxApi = new SandboxApi(core, invoker);
        }

        /// <summary>
        /// 成功すると、レスポンスが返り、authorization に Bearer トークンが付いている。
        /// </summary>
        [UnityTest]
        [Description("成功すると、レスポンスが返り、authorization に Bearer トークンが付いている")]
        public IEnumerator SuccessReturnsResponseWithBearerToken() => UniTask.ToCoroutine(async () =>
        {
            invoker.Register<PingRequest, PingResponse>(SandboxServiceMethods.Ping, (_, _) => UniTask.FromResult(Pinged("player-1")));

            var result = await sandboxApi.PingAsync(new PingRequest());

            Assert.That(result.IsSuccess, Is.True);
            Assert.That(result.Value.PlayerId, Is.EqualTo("player-1"));
            Assert.That(invoker.Calls.Single().GetHeader("authorization"), Is.EqualTo("Bearer token-1"));
        });

        /// <summary>
        /// UNAUTHENTICATED が返ると、トークンを発行し直して1回だけ送り直し、成功すればエラーハンドラーは呼ばれない。
        /// </summary>
        [UnityTest]
        [Description("UNAUTHENTICATED が返ると、トークンを発行し直して1回だけ送り直し、成功すればエラーハンドラーは呼ばれない")]
        public IEnumerator UnauthenticatedReissuesTokenAndResendsOnce() => UniTask.ToCoroutine(async () =>
        {
            invoker.Register<PingRequest, PingResponse>(SandboxServiceMethods.Ping, (_, headers) =>
                headers.GetValue("authorization") == "Bearer token-1"
                    ? throw FakeRpcErrors.Status(StatusCode.Unauthenticated)
                    : UniTask.FromResult(Pinged("player-1")));

            var result = await sandboxApi.PingAsync(new PingRequest());

            Assert.That(result.IsSuccess, Is.True);
            Assert.That(invoker.Calls.Select(call => call.GetHeader("authorization")),
                Is.EqualTo(new[] { "Bearer token-1", "Bearer token-2" }));
            Assert.That(errorHandler.Errors, Is.Empty);
        });

        /// <summary>
        /// 送り直しても UNAUTHENTICATED なら、それ以上は送らずにエラーハンドラーに渡す。
        /// </summary>
        [UnityTest]
        [Description("送り直しても UNAUTHENTICATED なら、それ以上は送らずにエラーハンドラーに渡す")]
        public IEnumerator UnauthenticatedTwiceGoesToErrorHandler() => UniTask.ToCoroutine(async () =>
        {
            invoker.Register<PingRequest, PingResponse>(SandboxServiceMethods.Ping, (_, _) =>
                throw FakeRpcErrors.Status(StatusCode.Unauthenticated));

            var result = await sandboxApi.PingAsync(new PingRequest());

            Assert.That(result.IsSuccess, Is.False);
            Assert.That(invoker.Calls, Has.Count.EqualTo(2));
            Assert.That(errorHandler.Errors.Single().Code, Is.EqualTo(ReservedErrors.UnknownError));
        });

        /// <summary>
        /// handledCodes に入れた業務エラーは、エラーハンドラーを通らずに、そのコードの失敗として返る。
        /// </summary>
        [UnityTest]
        [Description("handledCodes に入れた業務エラーは、エラーハンドラーを通らずに、そのコードの失敗として返る")]
        public IEnumerator HandledCodeReturnsWithoutErrorHandler() => UniTask.ToCoroutine(async () =>
        {
            invoker.Register<PingRequest, PingResponse>(SandboxServiceMethods.Ping, (_, _) =>
                throw FakeRpcErrors.FailedPrecondition(SandboxErrors.PlayerNotFound));

            var result = await sandboxApi.PingAsync(new PingRequest(), new[] { SandboxErrors.PlayerNotFound });

            Assert.That(result.IsSuccess, Is.False);
            Assert.That(result.Error.Code, Is.EqualTo(SandboxErrors.PlayerNotFound));
            Assert.That(result.Error.Code.Reason, Is.EqualTo("PLAYER_NOT_FOUND"));
            Assert.That(errorHandler.Errors, Is.Empty);
            Assert.Throws<InvalidOperationException>(() => _ = result.Value);
        });

        /// <summary>
        /// 予約コードは handledCodes に入れても、エラーハンドラーが処理する。
        /// </summary>
        [UnityTest]
        [Description("予約コードは handledCodes に入れても、エラーハンドラーが処理する")]
        public IEnumerator ReservedCodeGoesToErrorHandlerEvenIfHandled() => UniTask.ToCoroutine(async () =>
        {
            invoker.Register<PingRequest, PingResponse>(SandboxServiceMethods.Ping, (_, _) =>
                throw FakeRpcErrors.Status(StatusCode.Internal));

            var result = await sandboxApi.PingAsync(new PingRequest(), new[] { ReservedErrors.InternalError });

            Assert.That(result.IsSuccess, Is.False);
            Assert.That(errorHandler.Errors.Single().Code, Is.EqualTo(ReservedErrors.InternalError));
        });

        /// <summary>
        /// 受け取ったエラーは、表のとおりのエラーコードと、errors.toml の既定の action になる。
        /// </summary>
        [UnityTest]
        [Description("受け取ったエラーは、表のとおりのエラーコードと、errors.toml の既定の action になる")]
        public IEnumerator ErrorsAreConvertedToCodesAndDefaultActions() => UniTask.ToCoroutine(async () =>
        {
            var cases = new (RpcException Thrown, ErrorCode Code, ErrorAction Action)[]
            {
                (FakeRpcErrors.Status(StatusCode.Unavailable), ReservedErrors.NetworkError, ErrorAction.Retry),
                (FakeRpcErrors.Status(StatusCode.DeadlineExceeded), ReservedErrors.NetworkError, ErrorAction.Retry),
                (FakeRpcErrors.Status(StatusCode.Internal), ReservedErrors.InternalError, ErrorAction.Retry),
                (FakeRpcErrors.Status(StatusCode.Unimplemented), ReservedErrors.UnknownError, ErrorAction.Title),
                (FakeRpcErrors.Status(StatusCode.FailedPrecondition), ReservedErrors.UnknownError, ErrorAction.Title),
                (FakeRpcErrors.FailedPrecondition(ReservedErrors.Maintenance), ReservedErrors.Maintenance, ErrorAction.Title),
                (FakeRpcErrors.FailedPrecondition(ReservedErrors.UpdateRequired), ReservedErrors.UpdateRequired, ErrorAction.Update),
                (FakeRpcErrors.FailedPrecondition(SandboxErrors.SandboxStay), SandboxErrors.SandboxStay, ErrorAction.Stay),
                (FakeRpcErrors.FailedPrecondition(new ErrorCode(SandboxErrors.Domain, "NOT_IN_THIS_APP")), ReservedErrors.UnknownError, ErrorAction.Title),
                // reason が予約エラーと同じでも、domain が違えば知らないエラー
                (FakeRpcErrors.FailedPrecondition(new ErrorCode(SandboxErrors.Domain, "MAINTENANCE")), ReservedErrors.UnknownError, ErrorAction.Title),
            };

            foreach (var (thrown, code, action) in cases)
            {
                invoker.Register<PingRequest, PingResponse>(SandboxServiceMethods.Ping, (_, _) => throw thrown);

                var result = await sandboxApi.PingAsync(new PingRequest());

                Assert.That(result.Error.Code, Is.EqualTo(code), thrown.ToString());
                Assert.That(result.Error.Action, Is.EqualTo(action), thrown.ToString());
            }
        });

        /// <summary>
        /// トークンの取得で例外が起きると、CLIENT_ERROR としてエラーハンドラーに渡す。
        /// </summary>
        [UnityTest]
        [Description("トークンの取得で例外が起きると、CLIENT_ERROR としてエラーハンドラーに渡す")]
        public IEnumerator ClientExceptionIsClientError() => UniTask.ToCoroutine(async () =>
        {
            tokenProvider.ThrowOnGet = new InvalidOperationException("broken account file");

            var result = await sandboxApi.PingAsync(new PingRequest());

            Assert.That(result.Error.Code, Is.EqualTo(ReservedErrors.ClientError));
            Assert.That(errorHandler.Errors, Has.Count.EqualTo(1));
            Assert.That(invoker.Calls, Is.Empty);
        });

        /// <summary>
        /// 保存したアカウント情報が読めないと、CLIENT_ERROR ではなく ACCOUNT_DATA_CORRUPTED(既定の action は留まる)としてエラーハンドラーに渡す。
        /// </summary>
        [UnityTest]
        [Description("保存したアカウント情報が読めないと、CLIENT_ERROR ではなく ACCOUNT_DATA_CORRUPTED(既定の action は留まる)としてエラーハンドラーに渡す")]
        public IEnumerator CorruptedAccountDataIsAccountDataCorrupted() => UniTask.ToCoroutine(async () =>
        {
            tokenProvider.ThrowOnGet = new CorruptedAccountDataException("account.bin", null);

            var result = await sandboxApi.PingAsync(new PingRequest());

            Assert.That(result.Error.Code, Is.EqualTo(ReservedErrors.AccountDataCorrupted));
            Assert.That(result.Error.Action, Is.EqualTo(ErrorAction.Stay));
            Assert.That(errorHandler.Errors, Has.Count.EqualTo(1));
            Assert.That(invoker.Calls, Is.Empty);
        });

        /// <summary>
        /// 冪等性の対象の RPC は x-request-id を付け、エラーハンドラーが再試行を選ぶと同じ RequestID で送り直す。
        /// </summary>
        [UnityTest]
        [Description("冪等性の対象の RPC は x-request-id を付け、エラーハンドラーが再試行を選ぶと同じ RequestID で送り直す")]
        public IEnumerator RetryResendsWithSameRequestId() => UniTask.ToCoroutine(async () =>
        {
            invoker.Register<PingRequest, PingResponse>(SandboxServiceMethods.Ping, (_, _) =>
                invoker.Calls.Count == 1
                    ? throw FakeRpcErrors.Status(StatusCode.Unavailable)
                    : UniTask.FromResult(Pinged("player-1")));
            errorHandler.ThenReturn(ErrorDecision.Retry);

            var result = await InvokeIdempotencyTargetAsync();

            Assert.That(result.IsSuccess, Is.True);
            var requestIds = invoker.Calls.Select(call => call.GetHeader("x-request-id")).ToArray();
            Assert.That(requestIds, Has.Length.EqualTo(2));
            Assert.That(requestIds[0], Has.Length.EqualTo(32));
            Assert.That(requestIds[1], Is.EqualTo(requestIds[0]));
        });

        /// <summary>
        /// 冪等性の対象でない RPC には x-request-id を付けない。
        /// </summary>
        [UnityTest]
        [Description("冪等性の対象でない RPC には x-request-id を付けない")]
        public IEnumerator NonTargetHasNoRequestId() => UniTask.ToCoroutine(async () =>
        {
            invoker.Register<PingRequest, PingResponse>(SandboxServiceMethods.Ping, (_, _) => UniTask.FromResult(Pinged("player-1")));

            await sandboxApi.PingAsync(new PingRequest());

            Assert.That(invoker.Calls.Single().GetHeader("x-request-id"), Is.Null);
        });

        /// <summary>
        /// 冪等性の対象の RPC は1つずつ送り、前の応答を受け取るまで次を送らない。
        /// </summary>
        [UnityTest]
        [Description("冪等性の対象の RPC は1つずつ送り、前の応答を受け取るまで次を送らない")]
        public IEnumerator IdempotencyTargetsAreSentOneByOne() => UniTask.ToCoroutine(async () =>
        {
            var firstResponse = new UniTaskCompletionSource<PingResponse>();
            invoker.Register<PingRequest, PingResponse>(SandboxServiceMethods.Ping, (_, _) =>
                invoker.Calls.Count == 1 ? firstResponse.Task : UniTask.FromResult(Pinged("player-2")));

            var first = InvokeIdempotencyTargetAsync();
            var second = InvokeIdempotencyTargetAsync();
            await UniTask.Yield();
            Assert.That(invoker.Calls, Has.Count.EqualTo(1), "前の応答の前に次が送られた");

            firstResponse.TrySetResult(Pinged("player-1"));
            var (firstResult, secondResult) = await UniTask.WhenAll(first, second);

            Assert.That(firstResult.Value.PlayerId, Is.EqualTo("player-1"));
            Assert.That(secondResult.Value.PlayerId, Is.EqualTo("player-2"));
        });

        /// <summary>
        /// エラーハンドラーがタイトルに戻ると、順番待ちのリクエストは送らずに失敗になり、セッションも破棄される。
        /// </summary>
        [UnityTest]
        [Description("エラーハンドラーがタイトルに戻ると、順番待ちのリクエストは送らずに失敗になり、セッションも破棄される")]
        public IEnumerator ReturnToTitleDiscardsQueuedRequests() => UniTask.ToCoroutine(async () =>
        {
            invoker.Register<PingRequest, PingResponse>(SandboxServiceMethods.Ping, (_, _) =>
                throw FakeRpcErrors.Status(StatusCode.Internal));
            var dialogShown = new UniTaskCompletionSource();
            var closeDialog = new UniTaskCompletionSource();
            errorHandler.OnHandle = async () =>
            {
                dialogShown.TrySetResult();
                await closeDialog.Task;
            };
            errorHandler.ThenReturn(ErrorDecision.ReturnToTitle);

            var first = InvokeIdempotencyTargetAsync();
            await dialogShown.Task;
            var queued = InvokeIdempotencyTargetAsync();
            closeDialog.TrySetResult();
            var (firstResult, queuedResult) = await UniTask.WhenAll(first, queued);

            Assert.That(firstResult.IsSuccess, Is.False);
            Assert.That(queuedResult.Error.IsDiscarded, Is.True);
            Assert.That(invoker.Calls, Has.Count.EqualTo(1), "捨てたリクエストが送られた");
            Assert.That(tokenProvider.ResetCount, Is.EqualTo(1));
        });

        /// <summary>
        /// 成功すると、呼び出し側に返る前に、レスポンスが player_diff → バッジ → 通知の順に反映処理に渡されている。
        /// </summary>
        [UnityTest]
        [Description("成功すると、呼び出し側に返る前に、レスポンスが player_diff → バッジ → 通知の順に反映処理に渡されている")]
        public IEnumerator SuccessIsAppliedInOrderBeforeReturning() => UniTask.ToCoroutine(async () =>
        {
            var response = Pinged("player-1");
            invoker.Register<PingRequest, PingResponse>(SandboxServiceMethods.Ping, (_, _) => UniTask.FromResult(response));

            await sandboxApi.PingAsync(new PingRequest());

            Assert.That(applyOrder, Is.EqualTo(new[] { "player_diff", "badges", "notifications" }));
            Assert.That(diffApplier.Applied, Is.EqualTo(new[] { response }));
        });

        /// <summary>
        /// 失敗したときは、反映処理は呼ばれない。
        /// </summary>
        [UnityTest]
        [Description("失敗したときは、反映処理は呼ばれない")]
        public IEnumerator FailureIsNotApplied() => UniTask.ToCoroutine(async () =>
        {
            invoker.Register<PingRequest, PingResponse>(SandboxServiceMethods.Ping, (_, _) =>
                throw FakeRpcErrors.FailedPrecondition(SandboxErrors.PlayerNotFound));

            await sandboxApi.PingAsync(new PingRequest(), new[] { SandboxErrors.PlayerNotFound });

            Assert.That(applyOrder, Is.Empty);
        });

        /// <summary>
        /// 反映処理で例外が起きると、残りの反映処理は呼ばず、送り直さずに CLIENT_ERROR としてエラーハンドラーに渡して失敗を返し、順番待ちも解放する。
        /// </summary>
        [UnityTest]
        [Description("反映処理で例外が起きると、残りの反映処理は呼ばず、送り直さずに CLIENT_ERROR としてエラーハンドラーに渡して失敗を返し、順番待ちも解放する")]
        public IEnumerator ApplyExceptionIsClientErrorWithoutResend() => UniTask.ToCoroutine(async () =>
        {
            invoker.Register<PingRequest, PingResponse>(SandboxServiceMethods.Ping, (_, _) => UniTask.FromResult(Pinged("player-1")));
            badgeApplier.ThrowOnApply = new InvalidOperationException("bug in applier");
            // ハンドラーが再試行を選んでも、反映処理の失敗では送り直さない
            errorHandler.ThenReturn(ErrorDecision.Retry);

            var failed = await InvokeIdempotencyTargetAsync();

            Assert.That(failed.IsSuccess, Is.False);
            Assert.That(failed.Error.Code, Is.EqualTo(ReservedErrors.ClientError));
            Assert.That(errorHandler.Errors.Single().Code, Is.EqualTo(ReservedErrors.ClientError));
            Assert.That(invoker.Calls, Has.Count.EqualTo(1), "反映処理の失敗で送り直した");
            Assert.That(applyOrder, Is.EqualTo(new[] { "player_diff", "badges" }), "例外の後の反映処理を呼んだ");

            badgeApplier.ThrowOnApply = null;
            var next = await InvokeIdempotencyTargetAsync();
            Assert.That(next.IsSuccess, Is.True, "順番待ちが解放されていない");
        });

        /// <summary>
        /// 通信中にセッションを捨てると、後から届いたレスポンスは反映せず、捨てたものとして失敗を返す。
        /// </summary>
        [UnityTest]
        [Description("通信中にセッションを捨てると、後から届いたレスポンスは反映せず、捨てたものとして失敗を返す")]
        public IEnumerator ResponseAfterResetSessionIsNotApplied() => UniTask.ToCoroutine(async () =>
        {
            var response = new UniTaskCompletionSource<PingResponse>();
            invoker.Register<PingRequest, PingResponse>(SandboxServiceMethods.Ping, (_, _) => response.Task);

            var pending = sandboxApi.PingAsync(new PingRequest());
            await UniTask.Yield();
            core.ResetSession();
            response.TrySetResult(Pinged("old-player"));
            var result = await pending;

            Assert.That(result.IsSuccess, Is.False);
            Assert.That(result.Error.IsDiscarded, Is.True);
            Assert.That(applyOrder, Is.Empty);
        });

        /// <summary>
        /// タイトルに戻るときは、遷移より先にセッションを破棄し、順番待ちも解放してから遷移するので、
        /// 遷移の途中に送った API(冪等性の対象を含む)は新しいセッションで通る。
        /// </summary>
        [UnityTest]
        [Description("タイトルに戻るときは、遷移より先にセッションを破棄し、順番待ちも解放してから遷移するので、遷移の途中に送った API は新しいセッションで通る")]
        public IEnumerator ApiCalledDuringNavigationUsesNewSession() => UniTask.ToCoroutine(async () =>
        {
            var navigator = new RecordingNavigator();
            core = ApiCore.CreateBuilder(
                    new ApiCoreOptions(),
                    tokenProvider,
                    new ErrorPolicyProvider(SandboxErrors.All),
                    errorHandler)
                .WithPlayerDiffApplier(diffApplier)
                .WithErrorNavigator(navigator)
                .Build();
            sandboxApi = new SandboxApi(core, invoker);
            invoker.Register<PingRequest, PingResponse>(SandboxServiceMethods.Ping, (_, _) =>
                invoker.Calls.Count == 1 ? throw FakeRpcErrors.Status(StatusCode.Internal) : UniTask.FromResult(Pinged("player-1")));
            errorHandler.ThenReturn(ErrorDecision.ReturnToTitle);

            var resetCountAtNavigation = -1;
            var pingDuringNavigation = default(ApiResult<PingResponse>);
            var targetDuringNavigation = default(ApiResult<PingResponse>);
            navigator.OnGoToTitle = async () =>
            {
                resetCountAtNavigation = tokenProvider.ResetCount;
                pingDuringNavigation = await sandboxApi.PingAsync(new PingRequest());
                targetDuringNavigation = await InvokeIdempotencyTargetAsync();
            };

            var failed = await InvokeIdempotencyTargetAsync();

            Assert.That(failed.IsSuccess, Is.False);
            Assert.That(navigator.Navigated, Is.EqualTo(new[] { "title" }));
            Assert.That(resetCountAtNavigation, Is.EqualTo(1), "遷移の前にセッションが破棄されていない");
            Assert.That(pingDuringNavigation.IsSuccess, Is.True, "遷移の途中に送った API が捨てられた");
            Assert.That(targetDuringNavigation.IsSuccess, Is.True, "遷移の途中に送った冪等性の対象の API が通らない");
        });

        /// <summary>
        /// セッションを捨てると、応答を待っている通信は締め切りを待たずに止まり、ダイアログを出さずに捨てたものとして失敗を返す。
        /// </summary>
        [UnityTest]
        [Description("セッションを捨てると、応答を待っている通信は締め切りを待たずに止まり、ダイアログを出さずに捨てたものとして失敗を返す")]
        public IEnumerator ResetSessionCancelsPendingCall() => UniTask.ToCoroutine(async () =>
        {
            var neverResponds = new UniTaskCompletionSource<PingResponse>();
            invoker.Register<PingRequest, PingResponse>(SandboxServiceMethods.Ping, (_, _) => neverResponds.Task);

            var pending = sandboxApi.PingAsync(new PingRequest());
            await UniTask.Yield();
            core.ResetSession();
            var result = await pending;

            Assert.That(result.IsSuccess, Is.False);
            Assert.That(result.Error.IsDiscarded, Is.True);
            Assert.That(errorHandler.Errors, Is.Empty, "止めた通信でダイアログを出した");
        });

        /// <summary>
        /// エラーの詳細(grpc-status-details-bin)が壊れていても、例外にならずに UNKNOWN_ERROR としてエラーハンドラーに渡る。
        /// </summary>
        [UnityTest]
        [Description("エラーの詳細(grpc-status-details-bin)が壊れていても、例外にならずに UNKNOWN_ERROR としてエラーハンドラーに渡る")]
        public IEnumerator BrokenErrorDetailsBecomeUnknownError() => UniTask.ToCoroutine(async () =>
        {
            // 長さ5の項目と言いながら中身が無い、読めない google.rpc.Status
            var brokenDetails = new Metadata { { "grpc-status-details-bin", new byte[] { 0x0A, 0x05 } } };
            invoker.Register<PingRequest, PingResponse>(SandboxServiceMethods.Ping, (_, _) =>
                throw new RpcException(new Status(StatusCode.FailedPrecondition, "broken details"), brokenDetails));

            var result = await sandboxApi.PingAsync(new PingRequest());

            Assert.That(result.IsSuccess, Is.False);
            Assert.That(result.Error.Code, Is.EqualTo(ReservedErrors.UnknownError));
            Assert.That(errorHandler.Errors.Single().Code, Is.EqualTo(ReservedErrors.UnknownError));
        });

        private UniTask<ApiResult<PingResponse>> InvokeIdempotencyTargetAsync()
        {
            return new IdempotentPingApi(core, invoker).PingAsync();
        }

        private static PingResponse Pinged(string playerId)
        {
            return new PingResponse { PlayerId = playerId };
        }

        // 冪等性の対象として扱うかはクライアントの設定で決まるので、Ping を対象として送るラッパーで確かめる
        private sealed class IdempotentPingApi : ApiServiceBase
        {
            private static readonly RpcSettings settings =
                new(SandboxServiceMethods.Ping, isIdempotencyTarget: true, requiresAuth: true);

            private readonly SandboxService.SandboxServiceClient client;

            public IdempotentPingApi(ApiCore core, CallInvoker callInvoker) : base(core)
            {
                client = new SandboxService.SandboxServiceClient(callInvoker);
            }

            public UniTask<ApiResult<PingResponse>> PingAsync()
            {
                return InvokeAsync(settings, client, new PingRequest(), null, static (c, r, options) => c.PingAsync(r, options));
            }
        }
    }
}
