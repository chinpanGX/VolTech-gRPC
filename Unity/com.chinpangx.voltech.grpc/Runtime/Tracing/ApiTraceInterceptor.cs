using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.Threading.Tasks;
using Google.Protobuf;
using Grpc.Core;
using Grpc.Core.Interceptors;
using VolTech.Grpc.Client;

namespace VolTech.Grpc.Client.Tracing
{
    /// <summary>
    /// 単項の通信を ApiTraceStore に記録する Interceptor。記録するだけで、リクエストもレスポンスも変えない。
    /// </summary>
    /// <remarks>
    /// ApiCore の外(gRPC の層)で記録するので、ApiCore を通らないトークンの発行も、実際に送ったヘッダーも記録できる。
    /// </remarks>
    public sealed class ApiTraceInterceptor : Interceptor
    {
        // 既定値のフィールドも出して、送った・受け取ったことが分かるようにする。grpcurl・Postman にそのまま貼れる形
        private static readonly JsonFormatter formatter =
            new(JsonFormatter.Settings.Default.WithFormatDefaultValues(true).WithIndentation());

        private readonly ApiTraceStore store;

        /// <summary>
        /// 記録先を受け取って作る。
        /// </summary>
        public ApiTraceInterceptor(ApiTraceStore store)
        {
            this.store = store;
        }

        /// <summary>
        /// 送る前に「送信中」として記録し、結果を受け取ったら同じ Id の記録を埋めて置き換える。
        /// </summary>
        /// <remarks>
        /// 記録の処理(JSON への変換、記録の置き場所の購読者など)で例外が起きても、通信は失敗させない。
        /// 記録できなかったことは警告のログに出し、その通信は記録せずにそのまま送る。
        /// </remarks>
        public override AsyncUnaryCall<TResponse> AsyncUnaryCall<TRequest, TResponse>(
            TRequest request,
            ClientInterceptorContext<TRequest, TResponse> context,
            AsyncUnaryCallContinuation<TRequest, TResponse> continuation)
        {
            if (!store.Recording)
            {
                return continuation(request, context);
            }

            ApiTrace pending;
            try
            {
                pending = new ApiTrace
                {
                    Id = store.NextId(),
                    Method = context.Method.FullName,
                    StartedAt = DateTime.Now.ToString("o", CultureInfo.InvariantCulture),
                    Deadline = context.Options.Deadline is { } deadline && deadline != DateTime.MaxValue
                        ? deadline.ToUniversalTime().ToString("o", CultureInfo.InvariantCulture)
                        : "",
                    State = ApiTraceState.Pending,
                    RequestHeaders = ToHeaders(context.Options.Headers),
                    RequestBody = ToJson(request),
                };
                store.Add(pending);
            }
            catch (Exception exception)
            {
                LogRecordError(context.Method.FullName, exception);
                return continuation(request, context);
            }

            var stopwatch = Stopwatch.StartNew();
            var call = continuation(request, context);
            return new AsyncUnaryCall<TResponse>(
                RecordAsync(call, pending, stopwatch),
                call.ResponseHeadersAsync,
                call.GetStatus,
                call.GetTrailers,
                call.Dispose);
        }

        // レスポンスを待って記録し、結果(例外を含む)をそのまま返す。例外は包まずに投げ直す。
        // 待つ処理と記録する処理を分け、記録で起きた例外は呼び出し側に出さない
        private async Task<TResponse> RecordAsync<TResponse>(
            AsyncUnaryCall<TResponse> call,
            ApiTrace pending,
            Stopwatch stopwatch)
        {
            TResponse response;
            try
            {
                response = await call.ResponseAsync;
            }
            catch (RpcException exception)
            {
                var responseHeaders = await TryGetResponseHeadersAsync(call);
                TryRecord(pending, () =>
                {
                    var failed = pending.Clone();
                    failed.DurationMs = stopwatch.Elapsed.TotalMilliseconds;
                    failed.State = ApiTraceState.Failed;
                    failed.ResponseHeaders = responseHeaders;
                    failed.Trailers = ToHeaders(exception.Trailers);
                    failed.StatusCode = exception.StatusCode.ToString();
                    failed.StatusDetail = exception.Status.Detail;
                    FillErrorInfo(failed, exception);
                    store.Update(failed);
                });
                throw;
            }
            catch (Exception exception)
            {
                // 通信の外の例外(キャンセルなど)も、何が起きたかを残す
                TryRecord(pending, () =>
                {
                    var failed = pending.Clone();
                    failed.DurationMs = stopwatch.Elapsed.TotalMilliseconds;
                    failed.State = ApiTraceState.Failed;
                    failed.StatusCode = exception.GetType().Name;
                    failed.StatusDetail = exception.Message;
                    store.Update(failed);
                });
                throw;
            }

            var responseHeadersOnSuccess = await TryGetResponseHeadersAsync(call);
            TryRecord(pending, () =>
            {
                var succeeded = pending.Clone();
                succeeded.DurationMs = stopwatch.Elapsed.TotalMilliseconds;
                succeeded.State = ApiTraceState.Succeeded;
                succeeded.ResponseHeaders = responseHeadersOnSuccess;
                succeeded.ResponseBody = ToJson(response);
                succeeded.Trailers = ToHeaders(call.GetTrailers());
                var status = call.GetStatus();
                succeeded.StatusCode = status.StatusCode.ToString();
                succeeded.StatusDetail = status.Detail;
                store.Update(succeeded);
            });
            return response;
        }

        // 記録の処理で起きた例外は、通信の結果に混ぜずにログに出す
        private static void TryRecord(ApiTrace pending, Action record)
        {
            try
            {
                record();
            }
            catch (Exception exception)
            {
                LogRecordError(pending.Method, exception);
            }
        }

        private static void LogRecordError(string method, Exception exception)
        {
            UnityEngine.Debug.LogWarning($"[ApiTrace] {method} を記録できませんでした(通信には影響しません): {exception}");
        }

        // ErrorInfo を読む。壊れていても、記録のために通信を失敗させない
        private static void FillErrorInfo(ApiTrace trace, RpcException exception)
        {
            try
            {
                var errorInfo = RpcErrorDetails.GetErrorInfo(exception);
                if (errorInfo == null)
                {
                    return;
                }

                trace.ErrorReason = errorInfo.Reason;
                trace.ErrorDomain = errorInfo.Domain;
                foreach (var pair in errorInfo.Metadata)
                {
                    trace.ErrorMetadata.Add(new ApiTraceHeader(pair.Key, pair.Value));
                }
            }
            catch (Exception readError)
            {
                trace.ErrorReason = $"(could not read ErrorInfo: {readError.Message})";
            }
        }

        // エラーのときは、ヘッダーを受け取る前に失敗していることがある
        private static async Task<List<ApiTraceHeader>> TryGetResponseHeadersAsync<TResponse>(AsyncUnaryCall<TResponse> call)
        {
            try
            {
                return ToHeaders(await call.ResponseHeadersAsync);
            }
            catch (Exception)
            {
                return new List<ApiTraceHeader>();
            }
        }

        private static List<ApiTraceHeader> ToHeaders(Metadata metadata)
        {
            var headers = new List<ApiTraceHeader>();
            if (metadata == null)
            {
                return headers;
            }

            foreach (var entry in metadata)
            {
                var value = entry.IsBinary
                    ? $"({entry.ValueBytes.Length} bytes) {Convert.ToBase64String(entry.ValueBytes)}"
                    : entry.Value;
                headers.Add(new ApiTraceHeader(entry.Key, value));
            }

            return headers;
        }

        private static string ToJson(object message)
        {
            return message is IMessage protobuf ? formatter.Format(protobuf) : message?.ToString() ?? "";
        }
    }
}
