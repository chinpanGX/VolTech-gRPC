using System;
using System.Collections.Generic;
using System.Threading;
using System.Threading.Tasks;
using Cysharp.Threading.Tasks;
using Grpc.Core;

namespace VolTech.Client
{
    /// <summary>
    /// 通信する代わりに、RPC ごとに登録した処理を呼ぶ CallInvoker。サーバーが無いときの差し替えとテストに使う。
    /// </summary>
    /// <remarks>
    /// 生成されたクライアントに渡して使う。ラッパー・ApiCore・エラー処理は本物と同じように動く。
    /// 登録した処理で RpcException を投げると、サーバーがエラーを返したのと同じになる(FakeRpcErrors で作れる)。
    /// </remarks>
    public sealed class FakeCallInvoker : CallInvoker
    {
        private readonly Dictionary<string, Func<object, Metadata, UniTask<object>>> handlers = new();

        private readonly List<FakeCall> calls = new();

        /// <summary>これまでに呼ばれた RPC。呼ばれた順に並ぶ。</summary>
        public IReadOnlyList<FakeCall> Calls => calls;

        /// <summary>
        /// 全 RPC のレスポンスヘッダー(x-game-date など)。呼ばれた時点の中身を返すので、呼び出しの間に書き換えられる。
        /// </summary>
        public Metadata ResponseHeaders { get; } = new();

        /// <summary>
        /// RPC の処理を登録する。methodFullName は "/パッケージ.サービス/メソッド"(例: /app.v1.PlayerService/SignIn)。
        /// </summary>
        public FakeCallInvoker Register<TRequest, TResponse>(
            string methodFullName,
            Func<TRequest, Metadata, UniTask<TResponse>> handler)
        {
            handlers[methodFullName] = async (request, headers) => await handler((TRequest)request, headers);
            return this;
        }

        /// <summary>
        /// 登録した処理を呼ぶ。登録していない RPC は UNIMPLEMENTED になる。
        /// </summary>
        public override AsyncUnaryCall<TResponse> AsyncUnaryCall<TRequest, TResponse>(
            Method<TRequest, TResponse> method,
            string host,
            CallOptions options,
            TRequest request)
        {
            calls.Add(new FakeCall(method.FullName, request, options.Headers ?? new Metadata()));
            var responseTask = InvokeAsync<TResponse>(method.FullName, request, options.Headers, options.CancellationToken);
            var responseHeaders = new Metadata();
            foreach (var entry in ResponseHeaders)
            {
                responseHeaders.Add(entry);
            }

            return new AsyncUnaryCall<TResponse>(
                responseTask,
                Task.FromResult(responseHeaders),
                () => responseTask.IsFaulted && responseTask.Exception?.InnerException is RpcException exception
                    ? exception.Status
                    : Status.DefaultSuccess,
                () => new Metadata(),
                () => { });
        }

        /// <summary>
        /// 対応しない(単項の非同期呼び出しだけに対応する)。
        /// </summary>
        public override TResponse BlockingUnaryCall<TRequest, TResponse>(
            Method<TRequest, TResponse> method,
            string host,
            CallOptions options,
            TRequest request)
        {
            throw new NotSupportedException("FakeCallInvoker は単項の非同期呼び出しだけに対応する");
        }

        /// <summary>
        /// 対応しない(単項の非同期呼び出しだけに対応する)。
        /// </summary>
        public override AsyncServerStreamingCall<TResponse> AsyncServerStreamingCall<TRequest, TResponse>(
            Method<TRequest, TResponse> method,
            string host,
            CallOptions options,
            TRequest request)
        {
            throw new NotSupportedException("FakeCallInvoker は単項の非同期呼び出しだけに対応する");
        }

        /// <summary>
        /// 対応しない(単項の非同期呼び出しだけに対応する)。
        /// </summary>
        public override AsyncClientStreamingCall<TRequest, TResponse> AsyncClientStreamingCall<TRequest, TResponse>(
            Method<TRequest, TResponse> method,
            string host,
            CallOptions options)
        {
            throw new NotSupportedException("FakeCallInvoker は単項の非同期呼び出しだけに対応する");
        }

        /// <summary>
        /// 対応しない(単項の非同期呼び出しだけに対応する)。
        /// </summary>
        public override AsyncDuplexStreamingCall<TRequest, TResponse> AsyncDuplexStreamingCall<TRequest, TResponse>(
            Method<TRequest, TResponse> method,
            string host,
            CallOptions options)
        {
            throw new NotSupportedException("FakeCallInvoker は単項の非同期呼び出しだけに対応する");
        }

        // 本物の通信と同じく、CallOptions のキャンセルで止まったら CANCELLED にする(登録した処理の終わりを待たない)
        private async Task<TResponse> InvokeAsync<TResponse>(
            string methodFullName,
            object request,
            Metadata headers,
            CancellationToken cancellationToken)
        {
            if (!handlers.TryGetValue(methodFullName, out var handler))
            {
                throw new RpcException(new Status(StatusCode.Unimplemented, $"{methodFullName} is not registered"));
            }

            var response = handler(request, headers ?? new Metadata());
            if (!cancellationToken.CanBeCanceled)
            {
                return (TResponse)await response;
            }

            var cancelled = new UniTaskCompletionSource();
            using (cancellationToken.Register(() => cancelled.TrySetResult()))
            {
                var (completed, result) = await UniTask.WhenAny(response, cancelled.Task);
                if (!completed)
                {
                    throw new RpcException(new Status(StatusCode.Cancelled, $"{methodFullName} was cancelled"));
                }

                return (TResponse)result;
            }
        }
    }
}
