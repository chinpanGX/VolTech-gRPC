using System;
using System.Net.Sockets;
using VolTech.Client.Auth;
using Api.Badges;
using Api.Errors;
using Api.Notifications;
using Api.Repositories;
using VolTech.Client.Unity;
using Grpc.Core;
using Grpc.Net.Client;
using NUnit.Framework;
using Tests.EditMode.TestDoubles;
using VolTech.Client;

namespace Tests.EditMode
{
    /// <summary>
    /// 起動しているサーバー(make run)に、ApiCore を通してつなぐテストの土台。テストごとに新しい端末の ApiCore を作る。
    /// </summary>
    public abstract class ApiServerTestBase
    {
        private const string ServerHost = "localhost";
        private const int ServerPort = 50051;

        // サーバーが起動していないときに、テストが止まったままにならないようにする
        private static readonly TimeSpan connectTimeout = TimeSpan.FromSeconds(5);

        private GrpcChannel channel;

        /// <summary>サーバーにつながった CallInvoker。生成したラッパーに渡して使う。</summary>
        protected CallInvoker CallInvoker { get; private set; }

        /// <summary>アカウント情報の保存先。テストごとに空から始まる(新しい端末)。</summary>
        protected InMemoryAccountStore AccountStore { get; private set; }

        /// <summary>受け取ったエラーを記録するエラーハンドラー。何も足さなければ Stay を返す。</summary>
        protected RecordingErrorHandler ErrorHandler { get; private set; }

        /// <summary>ApiCore が player_diff を反映する、プレイヤーのデータ。</summary>
        protected PlayerRepository PlayerRepository { get; private set; }

        /// <summary>ApiCore が badges を反映する、バッジの状態。</summary>
        protected BadgeStore BadgeStore { get; private set; }

        /// <summary>ApiCore が notifications を積む、通知の待ち行列。</summary>
        protected NotificationQueue NotificationQueue { get; private set; }

        /// <summary>テストごとに作る ApiCore。</summary>
        protected ApiCore Core { get; private set; }

        /// <summary>
        /// サーバーが起動しているかを確かめ、起動していなければ理由が分かるメッセージで失敗させる。
        /// </summary>
        [OneTimeSetUp]
        public void EnsureServerIsRunning()
        {
            // 起動していないと gRPC の呼び出しは NETWORK_ERROR になるだけで、原因が読み取りにくいので先に確かめる
            using (var tcp = new TcpClient())
            {
                bool connected;
                try
                {
                    connected = tcp.ConnectAsync(ServerHost, ServerPort).Wait(connectTimeout);
                }
                catch (AggregateException)
                {
                    connected = false;
                }

                if (!connected)
                {
                    Assert.Fail($"サーバー({ServerHost}:{ServerPort})に接続できない。リポジトリのルートで make run を実行して、サーバーを起動してから実行する");
                }
            }
        }

        /// <summary>
        /// テストごとに、チャンネルと、新しい端末(アカウント未保存)の ApiCore を作る。
        /// </summary>
        [SetUp]
        public void CreateApiCore()
        {
            // アプリと同じ作り方(YetAnotherHttpHandler の HTTP/2)でつなぐ
            channel = ApiChannel.Create($"http://{ServerHost}:{ServerPort}");
            // 通信の Viewer に記録されるので、テストが失敗したときにやり取りを見返せる
            CallInvoker = ApiChannel.CreateCallInvoker(channel);

            // リポジトリの server_status.toml の最低バージョン(0)より新しいバージョンとして送る
            var options = new ApiCoreOptions(new AppVersion(AppVersion.Windows, 1));
            AccountStore = new InMemoryAccountStore();
            ErrorHandler = new RecordingErrorHandler();
            PlayerRepository = new PlayerRepository();
            BadgeStore = new BadgeStore();
            NotificationQueue = new NotificationQueue();
            var tokenProvider = new DeviceTokenProvider(CallInvoker, AccountStore, options);
            Core = new ApiCore(
                options,
                tokenProvider,
                ProtoErrorPolicyProvider.ForAppErrorCodes(),
                ErrorHandler,
                new PlayerDiffApplier(PlayerRepository),
                // 何をやり取りしたかを Console で見られるようにする
                new UnityApiLogger(),
                badgeApplier: new BadgeApplier(BadgeStore),
                notificationApplier: new NotificationApplier(NotificationQueue));
        }

        /// <summary>
        /// チャンネルを閉じる。
        /// </summary>
        [TearDown]
        public void DisposeChannel()
        {
            channel.Dispose();
        }
    }
}
