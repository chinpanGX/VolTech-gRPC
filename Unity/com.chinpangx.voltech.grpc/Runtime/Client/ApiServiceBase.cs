using System;
using System.Collections.Generic;
using Cysharp.Threading.Tasks;
using Grpc.Core;

namespace VolTech.Client
{
    /// <summary>
    /// 生成したラッパー(GachaApi など)の基底クラス。ラッパーは、ここの InvokeAsync を通して ApiCore で RPC を呼ぶ。
    /// </summary>
    /// <remarks>
    /// ラッパーはアプリのアセンブリに生成されるので、ApiCore.InvokeAsync は internal のまま、派生クラスにだけ protected で見せる。
    /// アプリはこのクラスを継承せず、生成したラッパーの XxxAsync を使う。
    /// </remarks>
    public abstract class ApiServiceBase
    {
        private readonly ApiCore core;

        /// <summary>
        /// RPC を呼ぶのに使う ApiCore を受け取って作る。
        /// </summary>
        /// <param name="core">共通処理。アプリで1つの ApiCore を、すべてのラッパーに渡す。</param>
        /// <exception cref="ArgumentNullException">core が null のとき。</exception>
        protected ApiServiceBase(ApiCore core)
        {
            this.core = core ?? throw new ArgumentNullException(nameof(core));
        }

        /// <summary>
        /// ApiCore で RPC を呼ぶ。失敗したときはエラーハンドラーの処理を済ませてから、失敗の結果を返す。
        /// </summary>
        /// <param name="settings">RPC ごとの設定。</param>
        /// <param name="client">生成された gRPC クライアント。</param>
        /// <param name="request">リクエスト。</param>
        /// <param name="handledCodes">呼び出し側が自分で処理するエラーコード。ダイアログを出さずに失敗として返る。</param>
        /// <param name="call">RPC を呼ぶ処理。キャプチャしない static ラムダで渡す。</param>
        /// <param name="notifyActivity">
        /// 進行中の呼び出しとしてアクティビティのハンドラーに知らせるか。画面を止めずに裏で呼ぶものは false にする。
        /// </param>
        /// <returns>呼び出しの結果。</returns>
        protected UniTask<ApiResult<TResponse>> InvokeAsync<TClient, TRequest, TResponse>(
            RpcSettings settings,
            TClient client,
            TRequest request,
            IEnumerable<ErrorCode> handledCodes,
            Func<TClient, TRequest, CallOptions, AsyncUnaryCall<TResponse>> call,
            bool notifyActivity = true)
        {
            return core.InvokeAsync(settings, client, request, handledCodes, call, notifyActivity);
        }

        /// <summary>
        /// RPC ごとの設定。生成したラッパーが .proto に合わせて持ち、InvokeAsync に渡す。
        /// </summary>
        /// <remarks>
        /// ラッパー(この型の派生クラス)と基盤の中だけで使うので、protected internal にしている。
        /// </remarks>
        protected internal sealed class RpcSettings
        {
            /// <summary>
            /// RPC の名前と、冪等性の対象か・認証が要るかを受け取って作る。
            /// </summary>
            /// <param name="methodName">RPC のパス(例: /app.v1.PlayerService/SignIn)。</param>
            /// <param name="isIdempotencyTarget">冪等性の対象(idempotency_level を書いていない)か。</param>
            /// <param name="requiresAuth">認証が要る(サービスに skip_auth が付いていない)か。</param>
            public RpcSettings(string methodName, bool isIdempotencyTarget, bool requiresAuth)
            {
                MethodName = methodName;
                IsIdempotencyTarget = isIdempotencyTarget;
                RequiresAuth = requiresAuth;
            }

            /// <summary>RPC のパス(例: /app.v1.PlayerService/SignIn)。ログに出す。</summary>
            public string MethodName { get; }

            /// <summary>
            /// 冪等性の対象(idempotency_level を書いていない)か。対象なら順番待ちをして、RequestID を付ける。
            /// </summary>
            public bool IsIdempotencyTarget { get; }

            /// <summary>認証が要る(サービスに skip_auth が付いていない)か。</summary>
            public bool RequiresAuth { get; }
        }
    }
}
