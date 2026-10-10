using System;
using System.Net.Sockets;
using Grpc.Core;
using Grpc.Net.Client;
using NUnit.Framework;
using Sandbox.Tests.EditMode.TestDoubles;
using Sandbox.V1;
using VolTech.Client;
using VolTech.Client.Auth;
using VolTech.Client.Unity;

namespace Sandbox.Tests.EditMode
{
    /// <summary>
    /// 起動している sandbox のサーバー(make sandbox-server)に、ApiCore を通してつなぐテストの土台。テストごとに新しい端末の ApiCore を作る。
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

        /// <summary>ApiCore が player_diff を反映する先。</summary>
        protected PlayerRepository Repository { get; private set; }

        /// <summary>ApiCore が badges を反映する先。</summary>
        protected BadgeStore Badges { get; private set; }

        /// <summary>ApiCore が notifications を積む先。</summary>
        protected NotificationQueue Notifications { get; private set; }

        /// <summary>ApiCore に渡したトークンの取得(デバイス登録とトークンの発行をする)。</summary>
        protected DeviceTokenProvider TokenProvider { get; private set; }

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
                    Assert.Fail($"サーバー({ServerHost}:{ServerPort})に接続できない。リポジトリのルートで make sandbox-server を実行して、サーバーを起動してから実行する");
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

            // sandbox の server_status.toml の最低バージョン(0)より新しいバージョンとして送る
            var options = new ApiCoreOptions(new AppVersion(AppVersion.Windows, 1));
            AccountStore = new InMemoryAccountStore();
            ErrorHandler = new RecordingErrorHandler();
            Repository = new PlayerRepository();
            Badges = new BadgeStore();
            Notifications = new NotificationQueue();
            TokenProvider = new DeviceTokenProvider(CallInvoker, AccountStore, options);
            Core = new ApiCore(
                options,
                TokenProvider,
                new ErrorPolicyProvider(SandboxErrors.All),
                ErrorHandler,
                new PlayerDiffApplier(Repository),
                // 何をやり取りしたかを Console で見られるようにする
                new UnityApiLogger(),
                badgeApplier: new BadgeApplier(Badges),
                notificationApplier: new NotificationApplier(Notifications));
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
