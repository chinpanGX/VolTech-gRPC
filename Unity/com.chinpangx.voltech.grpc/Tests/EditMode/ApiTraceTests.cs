using System.Collections;
using System.Collections.Generic;
using System.Linq;
using System.Text.RegularExpressions;
using App.V1;
using Cysharp.Threading.Tasks;
using Grpc.Core;
using Grpc.Core.Interceptors;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.TestTools;
using VolTech.Grpc.Client;
using VolTech.Grpc.Client.Tracing;

namespace Tests.EditMode
{
    /// <summary>
    /// 通信の Viewer のための記録(ApiTraceInterceptor・ApiTraceStore・実機から送る形)を、サーバーなしで確かめる。
    /// </summary>
    [Description("通信の Viewer のための記録(ApiTraceInterceptor・ApiTraceStore・実機から送る形)を、サーバーなしで確かめる")]
    public class ApiTraceTests
    {
        private ApiTraceStore store;
        private FakeCallInvoker invoker;
        private GachaService.GachaServiceClient client;

        /// <summary>
        /// テストごとに、空の記録の置き場所と、それに記録する偽のサーバーを作る。
        /// </summary>
        [SetUp]
        public void SetUp()
        {
            store = new ApiTraceStore();
            invoker = new FakeCallInvoker();
            client = new GachaService.GachaServiceClient(invoker.Intercept(new ApiTraceInterceptor(store)));
        }

        /// <summary>
        /// 成功した通信は、送ったヘッダー・Body(既定値も含む JSON)と、受け取ったヘッダー・Body が記録され、レスポンスはそのまま返る。
        /// </summary>
        [UnityTest]
        [Description("成功した通信は、送ったヘッダー・Body(既定値も含む JSON)と、受け取ったヘッダー・Body が記録され、レスポンスはそのまま返る")]
        public IEnumerator SuccessIsRecorded() => UniTask.ToCoroutine(async () =>
        {
            invoker.ResponseHeaders.Add("x-game-date", "2026-10-07");
            invoker.Register<DrawRequest, DrawResponse>(GachaServiceMethods.Draw, (_, _) =>
                UniTask.FromResult(new DrawResponse { Item = new PlayerItem { ItemId = 3, Count = 1 } }));

            var response = await client.DrawAsync(
                new DrawRequest(),
                new Metadata { { "authorization", "Bearer token-1" } });

            Assert.That(response.Item.ItemId, Is.EqualTo(3));
            var trace = store.Snapshot().Single();
            Assert.That(trace.Method, Is.EqualTo(GachaServiceMethods.Draw));
            Assert.That(trace.ShortMethod, Is.EqualTo("Draw"));
            Assert.That(trace.State, Is.EqualTo(ApiTraceState.Succeeded));
            Assert.That(trace.StatusCode, Is.EqualTo("OK"));
            Assert.That(trace.DurationMs, Is.GreaterThanOrEqualTo(0));
            Assert.That(Header(trace.RequestHeaders, "authorization"), Is.EqualTo("Bearer token-1"));
            // 既定値(free = false)も出すので、送ったことが分かる
            Assert.That(trace.RequestBody, Does.Contain("\"free\": false"));
            Assert.That(Header(trace.ResponseHeaders, "x-game-date"), Is.EqualTo("2026-10-07"));
            Assert.That(trace.ResponseBody, Does.Contain("\"itemId\": 3"));
        });

        /// <summary>
        /// 業務エラーは、ステータスと ErrorInfo(reason・metadata)が記録され、例外は包まずにそのまま呼び出し側に届く。
        /// </summary>
        [UnityTest]
        [Description("業務エラーは、ステータスと ErrorInfo(reason・metadata)が記録され、例外は包まずにそのまま呼び出し側に届く")]
        public IEnumerator BusinessErrorIsRecordedWithErrorInfo() => UniTask.ToCoroutine(async () =>
        {
            invoker.Register<DrawRequest, DrawResponse>(GachaServiceMethods.Draw, (_, _) =>
                throw FakeRpcErrors.FailedPrecondition(
                    "ERROR_CODE_MAINTENANCE",
                    "under maintenance",
                    new Dictionary<string, string> { ["end_at"] = "2026-10-07T06:00:00Z" }));

            RpcException thrown = null;
            try
            {
                await client.DrawAsync(new DrawRequest());
            }
            catch (RpcException exception)
            {
                thrown = exception;
            }

            Assert.That(thrown, Is.Not.Null);
            Assert.That(thrown.StatusCode, Is.EqualTo(StatusCode.FailedPrecondition));
            var trace = store.Snapshot().Single();
            Assert.That(trace.State, Is.EqualTo(ApiTraceState.Failed));
            Assert.That(trace.StatusCode, Is.EqualTo("FailedPrecondition"));
            Assert.That(trace.StatusDetail, Is.EqualTo("under maintenance"));
            Assert.That(trace.ErrorReason, Is.EqualTo("ERROR_CODE_MAINTENANCE"));
            Assert.That(Header(trace.ErrorMetadata, "end_at"), Is.EqualTo("2026-10-07T06:00:00Z"));
            // エラーの詳細はバイナリのトレーラーなので、バイト数と base64 で記録される
            Assert.That(Header(trace.Trailers, RpcErrorDetails.TrailerKey), Does.StartWith("("));
        });

        /// <summary>
        /// 記録を止めている間は、通信は通るが記録されない。
        /// </summary>
        [UnityTest]
        [Description("記録を止めている間は、通信は通るが記録されない")]
        public IEnumerator NotRecordedWhilePaused() => UniTask.ToCoroutine(async () =>
        {
            invoker.Register<DrawRequest, DrawResponse>(GachaServiceMethods.Draw, (_, _) => UniTask.FromResult(new DrawResponse()));
            store.Recording = false;

            await client.DrawAsync(new DrawRequest());

            Assert.That(store.Snapshot(), Is.Empty);
        });

        /// <summary>
        /// 記録の処理で例外が起きても(記録の置き場所の購読者が投げるなど)、通信は失敗せず、レスポンスがそのまま返る。
        /// </summary>
        [UnityTest]
        [Description("記録の処理で例外が起きても(記録の置き場所の購読者が投げるなど)、通信は失敗せず、レスポンスがそのまま返る")]
        public IEnumerator RecordingErrorDoesNotFailCall() => UniTask.ToCoroutine(async () =>
        {
            invoker.Register<DrawRequest, DrawResponse>(GachaServiceMethods.Draw, (_, _) =>
                UniTask.FromResult(new DrawResponse { Item = new PlayerItem { ItemId = 3, Count = 1 } }));
            store.Changed += _ => throw new System.InvalidOperationException("subscriber failed");
            LogAssert.Expect(LogType.Warning, new Regex(@"\[ApiTrace\].*subscriber failed"));

            var response = await client.DrawAsync(new DrawRequest());

            Assert.That(response.Item.ItemId, Is.EqualTo(3));
        });

        /// <summary>
        /// 同じ Id の記録は置き換わり、上限を超えたら古いものから捨てる。変わるたびに版が増える。
        /// </summary>
        [Test]
        [Description("同じ Id の記録は置き換わり、上限を超えたら古いものから捨てる。変わるたびに版が増える")]
        public void StoreReplacesByIdAndDropsOldest()
        {
            var version = store.Version;
            store.Add(new ApiTrace { Id = 1, State = ApiTraceState.Pending });
            Assert.That(store.Update(new ApiTrace { Id = 1, State = ApiTraceState.Succeeded }), Is.True);
            Assert.That(store.Snapshot().Single().State, Is.EqualTo(ApiTraceState.Succeeded));
            Assert.That(store.Version, Is.EqualTo(version + 2));

            for (var id = 2; id <= ApiTraceStore.Capacity + 1; id++)
            {
                store.Add(new ApiTrace { Id = id });
            }

            var traces = store.Snapshot();
            Assert.That(traces, Has.Length.EqualTo(ApiTraceStore.Capacity));
            Assert.That(traces.First().Id, Is.EqualTo(2));
            Assert.That(traces.Last().Id, Is.EqualTo(ApiTraceStore.Capacity + 1));
        }

        /// <summary>
        /// 送信中に消した記録(Clear・上限超え)は、結果を受け取って置き換えようとしても戻ってこない。
        /// </summary>
        [Test]
        [Description("送信中に消した記録(Clear・上限超え)は、結果を受け取って置き換えようとしても戻ってこない")]
        public void UpdateDoesNotRestoreDroppedTrace()
        {
            store.Add(new ApiTrace { Id = 1, State = ApiTraceState.Pending });
            store.Clear();

            Assert.That(store.Update(new ApiTrace { Id = 1, State = ApiTraceState.Succeeded }), Is.False);
            Assert.That(store.Snapshot(), Is.Empty);

            for (var id = 2; id <= ApiTraceStore.Capacity + 2; id++)
            {
                store.Add(new ApiTrace { Id = id });
            }

            Assert.That(store.Update(new ApiTrace { Id = 2, State = ApiTraceState.Succeeded }), Is.False);
            Assert.That(store.Snapshot().First().Id, Is.EqualTo(3));
        }

        /// <summary>
        /// 実機から送る形(JSON)にして読み戻すと、同じ中身になる。大きすぎる Body は切り詰め、Truncated になる。
        /// </summary>
        [Test]
        [Description("実機から送る形(JSON)にして読み戻すと、同じ中身になる。大きすぎる Body は切り詰め、Truncated になる")]
        public void MessageRoundTripsAndTruncatesLargeBody()
        {
            var trace = new ApiTrace
            {
                Id = 7,
                Method = GachaServiceMethods.Draw,
                State = ApiTraceState.Failed,
                RequestHeaders = { new ApiTraceHeader("x-request-id", "abc") },
                RequestBody = "{ \"free\": true }",
                ErrorReason = "ERROR_CODE_FREE_DRAW_USED",
                ErrorMetadata = { new ApiTraceHeader("action", "ERROR_ACTION_STAY") },
            };

            var decoded = ApiTraceMessages.Decode(ApiTraceMessages.Encode(trace));

            Assert.That(decoded.Id, Is.EqualTo(7));
            Assert.That(decoded.Method, Is.EqualTo(GachaServiceMethods.Draw));
            Assert.That(decoded.State, Is.EqualTo(ApiTraceState.Failed));
            Assert.That(Header(decoded.RequestHeaders, "x-request-id"), Is.EqualTo("abc"));
            Assert.That(decoded.RequestBody, Is.EqualTo(trace.RequestBody));
            Assert.That(Header(decoded.ErrorMetadata, "action"), Is.EqualTo("ERROR_ACTION_STAY"));
            Assert.That(decoded.Truncated, Is.False);

            trace.ResponseBody = new string('x', ApiTraceMessages.MaxBodyLength + 1);
            var truncated = ApiTraceMessages.Decode(ApiTraceMessages.Encode(trace));
            Assert.That(truncated.Truncated, Is.True);
            Assert.That(truncated.ResponseBody.Length, Is.LessThan(trace.ResponseBody.Length + 100));
            Assert.That(trace.Truncated, Is.False, "元の記録は変えない");

            // 切る位置がサロゲートペアの途中なら、その手前で切る(UTF-8 にしても化けない)
            trace.ResponseBody = new string('x', ApiTraceMessages.MaxBodyLength - 1) + "\U0001F600" + "tail";
            var surrogate = ApiTraceMessages.Decode(ApiTraceMessages.Encode(trace));
            Assert.That(surrogate.Truncated, Is.True);
            // Does.Contain \u306F\u30AB\u30EB\u30C1\u30E3\u4F9D\u5B58\u306E\u6BD4\u8F03\u3067 U+FFFD \u3092\u7121\u8996\u3057\u3066\u3057\u307E\u3046\u306E\u3067\u3001\u5E8F\u6570\u3067\u6BD4\u3079\u308B IndexOf(char) \u3092\u4F7F\u3046
            Assert.That(surrogate.ResponseBody.IndexOf('\uFFFD'), Is.EqualTo(-1));
            Assert.That(surrogate.ResponseBody.IndexOf('\uD83D'), Is.EqualTo(-1), "\u7D75\u6587\u5B57\u306E\u524D\u534A\u3060\u3051\u304C\u6B8B\u3063\u3066\u3044\u306A\u3044");
        }

        private static string Header(List<ApiTraceHeader> headers, string key)
        {
            return headers.FirstOrDefault(header => header.Key == key)?.Value;
        }
    }
}
