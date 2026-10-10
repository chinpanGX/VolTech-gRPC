using System;
using System.Threading;
using Cysharp.Net.Http;
using Grpc.Core;
using Grpc.Core.Interceptors;
using Grpc.Net.Client;

namespace VolTech.Client.Unity
{
    /// <summary>
    /// YetAnotherHttpHandler(HTTP/2)で通信する GrpcChannel を作る。チャンネルは1つを使い回す。
    /// </summary>
    public static class ApiChannel
    {
        // つながらないときに早めに諦め、裏で接続を試み続けるのを止める。YetAnotherHttpHandler は既定では無効
        private static readonly TimeSpan defaultConnectTimeout = TimeSpan.FromSeconds(10);

        private static readonly object gate = new();

        // AddInterceptor で足された Interceptor。足すたびに配列ごと差し替える(読む側は lock しない)
        private static Interceptor[] interceptors = Array.Empty<Interceptor>();

        /// <summary>
        /// 接続先の URL(例: http://localhost:50051)へのチャンネルを作る。
        /// </summary>
        public static GrpcChannel Create(string address, TimeSpan? connectTimeout = null)
        {
            var handler = new YetAnotherHttpHandler
            {
                // ローカルのサーバーは TLS なしの HTTP/2(h2c)で待ち受ける。指定しないと HTTP/1 になる
                Http2Only = address.StartsWith("http://", StringComparison.OrdinalIgnoreCase),
                ConnectTimeout = connectTimeout ?? defaultConnectTimeout,
            };
            return GrpcChannel.ForAddress(address, new GrpcChannelOptions
            {
                HttpHandler = handler,
                DisposeHttpClient = true,
            });
        }

        /// <summary>
        /// チャンネルで呼ぶ CallInvoker を作る。生成したラッパーと DeviceTokenProvider には、これを渡す。
        /// AddInterceptor で足された Interceptor を付ける。
        /// </summary>
        /// <remarks>
        /// エディタと開発ビルドでは、VolTech.Client.Tracing のアセンブリが記録の Interceptor を足す(通信の Viewer で見る)。
        /// リリースビルドには VolTech.Client.Tracing が入らないので、何も付かない。
        /// 付ける Interceptor は呼ぶたびに決めるので、CallInvoker を作った後に足された Interceptor も付く。
        /// </remarks>
        public static CallInvoker CreateCallInvoker(GrpcChannel channel)
        {
            return new RegisteredInterceptorsCallInvoker(channel.CreateCallInvoker());
        }

        /// <summary>
        /// この後に CreateCallInvoker で作る CallInvoker に、Interceptor を足す。開発用のツール(通信の記録など)が、起動時に呼ぶ。
        /// </summary>
        /// <remarks>
        /// Api は開発用のツールのアセンブリを参照しない(リリースビルドで消えるため)。ツールの側がここに自分を登録する。
        /// </remarks>
        public static void AddInterceptor(Interceptor interceptor)
        {
            lock (gate)
            {
                var added = new Interceptor[interceptors.Length + 1];
                interceptors.CopyTo(added, 0);
                added[interceptors.Length] = interceptor;
                Volatile.Write(ref interceptors, added);
            }
        }

        // 呼ぶたびに、その時点で足されている Interceptor を付けて呼ぶ CallInvoker
        private sealed class RegisteredInterceptorsCallInvoker : CallInvoker
        {
            private readonly CallInvoker inner;

            // 前回付けた Interceptor の配列と、それを付けた CallInvoker。配列が変わったときだけ作り直す
            private Resolved resolved;

            public RegisteredInterceptorsCallInvoker(CallInvoker inner)
            {
                this.inner = inner;
                resolved = new Resolved(Array.Empty<Interceptor>(), inner);
            }

            public override TResponse BlockingUnaryCall<TRequest, TResponse>(
                Method<TRequest, TResponse> method, string host, CallOptions options, TRequest request)
            {
                return Resolve().BlockingUnaryCall(method, host, options, request);
            }

            public override AsyncUnaryCall<TResponse> AsyncUnaryCall<TRequest, TResponse>(
                Method<TRequest, TResponse> method, string host, CallOptions options, TRequest request)
            {
                return Resolve().AsyncUnaryCall(method, host, options, request);
            }

            public override AsyncServerStreamingCall<TResponse> AsyncServerStreamingCall<TRequest, TResponse>(
                Method<TRequest, TResponse> method, string host, CallOptions options, TRequest request)
            {
                return Resolve().AsyncServerStreamingCall(method, host, options, request);
            }

            public override AsyncClientStreamingCall<TRequest, TResponse> AsyncClientStreamingCall<TRequest, TResponse>(
                Method<TRequest, TResponse> method, string host, CallOptions options)
            {
                return Resolve().AsyncClientStreamingCall(method, host, options);
            }

            public override AsyncDuplexStreamingCall<TRequest, TResponse> AsyncDuplexStreamingCall<TRequest, TResponse>(
                Method<TRequest, TResponse> method, string host, CallOptions options)
            {
                return Resolve().AsyncDuplexStreamingCall(method, host, options);
            }

            private CallInvoker Resolve()
            {
                var current = Volatile.Read(ref interceptors);
                var last = Volatile.Read(ref resolved);
                if (ReferenceEquals(last.Interceptors, current))
                {
                    return last.CallInvoker;
                }

                var next = new Resolved(current, current.Length == 0 ? inner : inner.Intercept(current));
                Volatile.Write(ref resolved, next);
                return next.CallInvoker;
            }

            private sealed class Resolved
            {
                public Resolved(Interceptor[] interceptors, CallInvoker callInvoker)
                {
                    Interceptors = interceptors;
                    CallInvoker = callInvoker;
                }

                public Interceptor[] Interceptors { get; }

                public CallInvoker CallInvoker { get; }
            }
        }
    }
}
