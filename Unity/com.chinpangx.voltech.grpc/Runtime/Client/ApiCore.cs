using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading;
using Cysharp.Threading.Tasks;
using Google.Protobuf;
using Grpc.Core;

namespace VolTech.Client
{
    /// <summary>
    /// API の呼び出しの共通処理。トークンとヘッダーの付与、順番待ち、再認証、エラーの変換とハンドラーの呼び出し、
    /// サーバー時刻の同期とゲーム内の日付の保持、共通レスポンス項目の反映を行う。
    /// </summary>
    /// <remarks>
    /// 処理の流れ: InvokeAsync(順番待ち) → SendWithRetryAsync(再認証・再送のループ) → SendOnceAsync(1回の送信)。
    /// 成功したら CompleteAsync で共通レスポンス項目を反映し、失敗したら HandleErrorAsync でエラーハンドラーに渡す。
    /// エラーハンドラーがタイトル・ホームに戻ると決めたら、先にセッションを終わらせ(古い通信を止め)、
    /// 順番待ちを解放してから画面を遷移させる。遷移先の画面は、遷移の途中からでも新しいセッションで API を呼べる。
    /// </remarks>
    public sealed class ApiCore
    {
        private readonly ApiCoreOptions options;
        private readonly ITokenProvider tokenProvider;
        private readonly IErrorPolicyProvider errorPolicy;
        private readonly ICommonErrorHandler errorHandler;
        private readonly IPlayerDiffApplier playerDiffApplier;
        private readonly IBadgeApplier badgeApplier;
        private readonly INotificationApplier notificationApplier;
        private readonly IErrorNavigator navigator;

        // null ならログを出さない。?. で呼ぶので、ログの文字列(リクエスト・レスポンスの整形)も作らない
        private readonly IApiLogger logger;
        private readonly ServerClock serverClock;

        // 冪等性の対象の RPC を1つずつ送るための順番待ち
        private readonly SemaphoreSlim requestQueue = new(1, 1);

        // エラーハンドラーを1つずつ呼ぶ(ダイアログを同時に出さない)ための順番待ち
        private readonly SemaphoreSlim errorHandlerQueue = new(1, 1);

        // セッションの世代。タイトル・ホームに戻ったり、セッションを破棄したりするたびに増やす。
        // 呼び出しを始めたときの世代と今の世代が違えば、古いセッションの呼び出しなので、送らない・反映しない
        private int sessionEpoch;

        // セッションの通信を止めるためのキャンセル。セッションを終わらせるたびに Cancel して作り直す。
        // 送信中の通信と順番待ちが、締め切りを待たずにすぐ抜ける
        private CancellationTokenSource sessionCancellation = new();

        // セッションの最初に受け取ったゲーム内の日付。以降のリクエストに付け、サーバーは日付が変わっていれば DATE_CHANGED を返す。
        // 受け取るたびに更新すると日付の変化に気づけないので、セッションを破棄するまで最初の値を使う
        private string gameDate;

        /// <summary>
        /// 共通処理に使う部品を受け取って作る。
        /// </summary>
        public ApiCore(
            ApiCoreOptions options,
            ITokenProvider tokenProvider,
            IErrorPolicyProvider errorPolicy,
            ICommonErrorHandler errorHandler,
            IPlayerDiffApplier playerDiffApplier = null,
            IApiLogger logger = null,
            ServerClock serverClock = null,
            IErrorNavigator navigator = null,
            IBadgeApplier badgeApplier = null,
            INotificationApplier notificationApplier = null)
        {
            this.options = options;
            this.tokenProvider = tokenProvider;
            this.errorPolicy = errorPolicy;
            this.errorHandler = errorHandler;
            this.playerDiffApplier = playerDiffApplier;
            this.logger = logger;
            this.serverClock = serverClock ?? new ServerClock();
            this.navigator = navigator;
            this.badgeApplier = badgeApplier;
            this.notificationApplier = notificationApplier;
        }

        /// <summary>
        /// 今のサーバー時刻。成功したレスポンスのヘッダーで受け取ったサーバー時刻から計算する。
        /// </summary>
        public IServerClock ServerClock => serverClock;

        /// <summary>
        /// セッションを破棄する。覚えているトークン・アカウント情報・ゲーム内の日付を捨て、送信中・順番待ちのリクエストを止めて失敗にする。
        /// </summary>
        /// <remarks>
        /// アカウントの引き継ぎ・データ削除で、保存したアカウント情報を入れ替えた後に呼ぶ。
        /// </remarks>
        public void ResetSession()
        {
            var previous = BeginNewSession();
            Volatile.Write(ref gameDate, null);
            tokenProvider.Reset();
            // 止めた通信の後続が動く前に、捨てるものをすべて捨てておく
            previous.Cancel();
        }

        /// <summary>
        /// RPC を呼ぶ。失敗したときはエラーハンドラーの処理を済ませてから、失敗の結果を返す。
        /// </summary>
        /// <param name="settings">RPC ごとの設定。</param>
        /// <param name="client">生成された gRPC クライアント。</param>
        /// <param name="request">リクエスト。</param>
        /// <param name="handledCodes">呼び出し側が自分で処理するエラーコード。ダイアログを出さずに失敗として返る。</param>
        /// <param name="call">RPC を呼ぶ処理。キャプチャしない static ラムダで渡す。</param>
        public async UniTask<ApiResult<TResponse>> InvokeAsync<TClient, TRequest, TResponse>(
            RpcSettings settings,
            TClient client,
            TRequest request,
            IEnumerable<int> handledCodes,
            Func<TClient, TRequest, CallOptions, AsyncUnaryCall<TResponse>> call)
        {
            // 世代を先に読む。BeginNewSession はキャンセルを入れ替えてから世代を進めるので、
            // 新しい世代を読んだなら、キャンセルも必ず新しいものになる
            var epoch = Volatile.Read(ref sessionEpoch);
            var cancellationToken = Volatile.Read(ref sessionCancellation).Token;
            if (settings.IsIdempotencyTarget)
            {
                // 応答を受け取る前に次を送ると、サーバーに保存された最新の RequestID が上書きされ、再送で二重実行になる
                try
                {
                    await requestQueue.WaitAsync(cancellationToken);
                }
                catch (OperationCanceledException)
                {
                    // 順番を待っている間にタイトル・ホームに戻ったので、送らずに捨てる
                    logger?.Log($"[Api] {settings.MethodName} discarded");
                    return ApiResult<TResponse>.Failure(ApiError.Discarded());
                }
            }

            (ApiResult<TResponse> Result, ErrorDecision Decision) outcome;
            try
            {
                if (IsStale(epoch))
                {
                    logger?.Log($"[Api] {settings.MethodName} discarded");
                    return ApiResult<TResponse>.Failure(ApiError.Discarded());
                }

                outcome = await SendWithRetryAsync(settings, client, request, handledCodes, call, epoch, cancellationToken);
            }
            finally
            {
                if (settings.IsIdempotencyTarget)
                {
                    requestQueue.Release();
                }
            }

            // 画面の遷移は、順番待ちを解放してから行う。遷移先の画面がすぐに API を呼んでも待たされない
            await NavigateAsync(outcome.Decision);
            return outcome.Result;
        }

        // 成功するか、諦めるまで送る。送り直すのは次の2つのときだけ。
        // - UNAUTHENTICATED: トークンを取り直して1回だけ自動で送り直す
        // - エラーハンドラーが Retry を返した: 同じ RequestID で送り直す
        // エラーハンドラーが決めた画面の遷移は、結果と一緒に返し、順番待ちを解放してから InvokeAsync が行う
        private async UniTask<(ApiResult<TResponse> Result, ErrorDecision Decision)> SendWithRetryAsync<TClient, TRequest, TResponse>(
            RpcSettings settings,
            TClient client,
            TRequest request,
            IEnumerable<int> handledCodes,
            Func<TClient, TRequest, CallOptions, AsyncUnaryCall<TResponse>> call,
            int epoch,
            CancellationToken cancellationToken)
        {
            // 再送では同じ RequestID を使うので、ループの外で採番する
            var requestId = settings.IsIdempotencyTarget ? Guid.NewGuid().ToString("N") : null;
            var reauthenticated = false;
            while (true)
            {
                var attempt = await SendOnceAsync(settings, client, request, call, requestId, canReauthenticate: !reauthenticated, cancellationToken);

                // 通信中にセッションを終わらせていたら(キャンセルで止まったときも)、結果を使わずに捨てる
                if (IsStale(epoch))
                {
                    logger?.Log($"[Api] {settings.MethodName} discarded after response");
                    return (ApiResult<TResponse>.Failure(ApiError.Discarded()), ErrorDecision.Stay);
                }

                if (attempt.Outcome == SendOutcome.Unauthenticated)
                {
                    // サーバーが処理を始める前に弾いたものなので、トークンを取り直して1回だけ送り直しても二重実行にならない
                    logger?.Log($"[Api] ← {settings.MethodName} UNAUTHENTICATED. reissue token and resend");
                    tokenProvider.Invalidate(attempt.Token);
                    reauthenticated = true;
                    continue;
                }

                if (attempt.Outcome == SendOutcome.Success)
                {
                    return await CompleteAsync(settings, attempt.Response, attempt.ResponseHeaders, epoch);
                }

                var error = attempt.Error;
                logger?.Log($"[Api] ← {settings.MethodName} {error}");

                if (IsHandledByCaller(error, handledCodes))
                {
                    return (ApiResult<TResponse>.Failure(error), ErrorDecision.Stay);
                }

                var decision = await HandleErrorAsync(error, epoch);
                if (decision == ErrorDecision.Retry)
                {
                    reauthenticated = false;
                    continue;
                }

                return (ApiResult<TResponse>.Failure(error), decision);
            }
        }

        // 1回だけ送る。例外は投げず、成功・再認証が要る・エラーのどれかを返す
        private async UniTask<SendAttempt<TResponse>> SendOnceAsync<TClient, TRequest, TResponse>(
            RpcSettings settings,
            TClient client,
            TRequest request,
            Func<TClient, TRequest, CallOptions, AsyncUnaryCall<TResponse>> call,
            string requestId,
            bool canReauthenticate,
            CancellationToken cancellationToken)
        {
            string token = null;
            try
            {
                if (settings.RequiresAuth)
                {
                    token = await tokenProvider.GetTokenAsync();
                }

                var callOptions = new CallOptions(
                    CreateHeaders(token, requestId),
                    DateTime.UtcNow.Add(options.Deadline),
                    cancellationToken);
                logger?.Log($"[Api] → {settings.MethodName}\n{request}");
                // Task は AsUniTask() を通さずにそのまま await する(本体の決まり)
                var rpc = call(client, request, callOptions);
                var response = await rpc.ResponseAsync;
                var responseHeaders = await rpc.ResponseHeadersAsync;
                logger?.Log($"[Api] ← {settings.MethodName} OK\n{response}");
                return SendAttempt<TResponse>.Success(response, responseHeaders);
            }
            catch (RpcException exception) when (
                exception.StatusCode == StatusCode.Unauthenticated && settings.RequiresAuth && canReauthenticate)
            {
                return SendAttempt<TResponse>.Unauthenticated(token);
            }
            catch (Exception exception)
            {
                return SendAttempt<TResponse>.Failure(ConvertError(settings, exception));
            }
        }

        // 受け取った例外をエラーコードにする。catch 句の中で変換すると、変換で起きた例外は他の catch 句で捕まらずに
        // 呼び出し側まで抜けるので、変換の失敗もここで CLIENT_ERROR にする
        private ApiError ConvertError(RpcSettings settings, Exception exception)
        {
            if (exception is RpcException rpcException)
            {
                try
                {
                    return ErrorConverter.FromRpcException(rpcException, errorPolicy);
                }
                catch (Exception conversionException)
                {
                    logger?.LogException($"[Api] {settings.MethodName} failed to convert {rpcException.Status}", conversionException);
                    return ErrorConverter.FromClientException(conversionException, errorPolicy);
                }
            }

            logger?.LogException($"[Api] {settings.MethodName} failed in client", exception);
            return ErrorConverter.FromClientException(exception, errorPolicy);
        }

        // 成功したレスポンスの後始末。古いセッションのものでなければ、共通レスポンス項目を反映して返す
        private async UniTask<(ApiResult<TResponse> Result, ErrorDecision Decision)> CompleteAsync<TResponse>(
            RpcSettings settings,
            TResponse response,
            Metadata responseHeaders,
            int epoch)
        {
            // 通信中にタイトル・ホームに戻ったりセッションを捨てたりしていたら、古いセッションのレスポンスなので
            // 反映しない。反映すると、その後にサインインで置き換えた新しいデータを上書きしてしまう
            if (IsStale(epoch))
            {
                logger?.Log($"[Api] {settings.MethodName} discarded after response");
                return (ApiResult<TResponse>.Failure(ApiError.Discarded()), ErrorDecision.Stay);
            }

            ApplyResponseHeaders(responseHeaders);

            // 反映処理の例外は、通信のエラーと違って送り直さないので、送信とは別に扱う
            var (applyError, decision) = await ApplyCommonFieldsAsync(settings, response, epoch);
            return applyError == null
                ? (ApiResult<TResponse>.Success(response), ErrorDecision.Stay)
                : (ApiResult<TResponse>.Failure(applyError), decision);
        }

        // 共通レスポンス項目を player_diff → バッジ → 通知の順に反映する。順番待ちの中で行うので、次の API が送られる時点で
        // 必ず反映が終わっている。反映処理で例外が起きたら、残りは反映せずに CLIENT_ERROR として共通エラーハンドラーに渡し、
        // そのエラーとハンドラーが決めた動きを返す
        private async UniTask<(ApiError Error, ErrorDecision Decision)> ApplyCommonFieldsAsync<TResponse>(
            RpcSettings settings,
            TResponse response,
            int epoch)
        {
            if (!(response is IMessage message))
            {
                return (null, ErrorDecision.Stay);
            }

            var applying = "player_diff";
            try
            {
                if (playerDiffApplier != null)
                {
                    await playerDiffApplier.ApplyAsync(message);
                }

                applying = "badges";
                if (badgeApplier != null)
                {
                    await badgeApplier.ApplyAsync(message);
                }

                applying = "notifications";
                if (notificationApplier != null)
                {
                    await notificationApplier.ApplyAsync(message);
                }

                return (null, ErrorDecision.Stay);
            }
            catch (Exception exception)
            {
                // 反映処理の例外はアプリの実装のバグ。サーバーの処理は成功しているので送り直さない。
                // タイトルに戻ってサインインで全件を取り直し、サーバーとの食い違いを解消してもらう
                logger?.LogException($"[Api] {settings.MethodName} failed to apply {applying}\n{response}", exception);
                var error = ErrorConverter.FromClientException(exception, errorPolicy);
                var decision = await HandleErrorAsync(error, epoch);
                return (error, decision);
            }
        }

        // 予約コードは画面ごとに扱いが変わらないように、handledCodes に入っていても共通エラーハンドラーに回す
        private static bool IsHandledByCaller(ApiError error, IEnumerable<int> handledCodes)
        {
            return !ReservedErrorCodes.IsReserved(error.Code) && handledCodes != null && handledCodes.Contains(error.Code);
        }

        private Metadata CreateHeaders(string token, string requestId)
        {
            var headers = new Metadata();
            if (token != null)
            {
                headers.Add(ApiHeaders.Authorization, $"Bearer {token}");
            }

            if (requestId != null)
            {
                headers.Add(ApiHeaders.RequestId, requestId);
            }

            options.AppVersion?.AddHeadersTo(headers);

            // サインアップ・サインインなど、セッションの最初の呼び出しでは日付をまだ持っていないので付けない
            var date = Volatile.Read(ref gameDate);
            if (date != null)
            {
                headers.Add(ApiHeaders.GameDate, date);
            }

            return headers;
        }

        // サーバー時刻を記録し、ゲーム内の日付をまだ持っていなければ覚える
        private void ApplyResponseHeaders(Metadata responseHeaders)
        {
            var serverTime = responseHeaders?.GetValue(ApiHeaders.ServerTime);
            if (long.TryParse(serverTime, out var unixMilliseconds))
            {
                serverClock.Synchronize(DateTimeOffset.FromUnixTimeMilliseconds(unixMilliseconds));
            }

            var date = responseHeaders?.GetValue(ApiHeaders.GameDate);
            if (!string.IsNullOrEmpty(date))
            {
                Interlocked.CompareExchange(ref gameDate, date, null);
            }
        }

        private async UniTask<ErrorDecision> HandleErrorAsync(ApiError error, int epoch)
        {
            await errorHandlerQueue.WaitAsync();
            try
            {
                // 前のダイアログでタイトル・ホームに戻っていたら、ダイアログを出さずに失敗にする
                if (IsStale(epoch))
                {
                    return ErrorDecision.Stay;
                }

                ErrorDecision decision;
                try
                {
                    decision = await errorHandler.HandleAsync(error);
                }
                catch (Exception exception)
                {
                    logger?.LogException("[Api] error handler failed", exception);
                    decision = ErrorDecision.Stay;
                }

                // 画面を遷移する前に、セッションを終わらせる(古い通信を止め、次のエラーのダイアログも出さない)。
                // 遷移は、順番待ちを解放してから InvokeAsync が行う
                switch (decision)
                {
                    case ErrorDecision.ReturnToHome:
                        BeginNewSession().Cancel();
                        break;
                    case ErrorDecision.ReturnToTitle:
                        ResetSession();
                        break;
                }

                return decision;
            }
            finally
            {
                errorHandlerQueue.Release();
            }
        }

        // 呼び出しを始めたときから、セッションの世代が変わったか
        private bool IsStale(int epoch)
        {
            return epoch != Volatile.Read(ref sessionEpoch);
        }

        // 新しいセッションにする。キャンセルを入れ替えてから世代を進め、止めるための古いキャンセルを返す。
        // 古いキャンセルは、まだそのトークンを持つ通信があるので Dispose しない(タイマーを持たないので GC で回収される)
        private CancellationTokenSource BeginNewSession()
        {
            var previous = Interlocked.Exchange(ref sessionCancellation, new CancellationTokenSource());
            Interlocked.Increment(ref sessionEpoch);
            return previous;
        }

        // エラーハンドラーが決めた画面の遷移をする。遷移しない動き(Retry・Stay)なら何もしない
        private async UniTask NavigateAsync(ErrorDecision decision)
        {
            if (navigator == null)
            {
                return;
            }

            try
            {
                switch (decision)
                {
                    case ErrorDecision.ReturnToTitle:
                        await navigator.GoToTitleAsync();
                        break;
                    case ErrorDecision.ReturnToHome:
                        await navigator.GoToHomeAsync();
                        break;
                    case ErrorDecision.OpenStore:
                        await navigator.OpenStoreAsync();
                        break;
                }
            }
            catch (Exception exception)
            {
                logger?.LogException($"[Api] navigation to {decision} failed", exception);
            }
        }

        private enum SendOutcome
        {
            Success,
            Unauthenticated,
            Failure,
        }

        // 1回の送信の結果。SendOnceAsync の中だけで使う
        private readonly struct SendAttempt<TResponse>
        {
            private SendAttempt(SendOutcome outcome, TResponse response, Metadata responseHeaders, string token, ApiError error)
            {
                Outcome = outcome;
                Response = response;
                ResponseHeaders = responseHeaders;
                Token = token;
                Error = error;
            }

            public SendOutcome Outcome { get; }

            // Success のときのレスポンス
            public TResponse Response { get; }

            // Success のときのレスポンスヘッダー
            public Metadata ResponseHeaders { get; }

            // Unauthenticated のときに、サーバーに弾かれたトークン
            public string Token { get; }

            // Failure のときのエラー
            public ApiError Error { get; }

            public static SendAttempt<TResponse> Success(TResponse response, Metadata responseHeaders)
            {
                return new SendAttempt<TResponse>(SendOutcome.Success, response, responseHeaders, null, null);
            }

            public static SendAttempt<TResponse> Unauthenticated(string token)
            {
                return new SendAttempt<TResponse>(SendOutcome.Unauthenticated, default, null, token, null);
            }

            public static SendAttempt<TResponse> Failure(ApiError error)
            {
                return new SendAttempt<TResponse>(SendOutcome.Failure, default, null, null, error);
            }
        }
    }
}
