using Grpc.Core;
using Grpc.Net.Client;
using Sandbox;
using Sandbox.V1;
using UnityEngine;
using VContainer;
using VContainer.Unity;
using VolTech.Client;
using VolTech.Client.Auth;
using VolTech.Client.Unity;

namespace Sample
{
    /// <summary>
    /// ガチャのサンプルの部品を組み立てる。基盤(ApiCore・認証・エラー処理・共通レスポンス項目の反映)と、画面の実装例をつなぐ。
    /// </summary>
    /// <remarks>
    /// VContainer は、名前が LifetimeScope.cs で終わる新しいスクリプトをひな形で上書きする(ScriptTemplateModifier)。
    /// このファイルを作り直すときは、meta ができてから中身を書くか、VContainerSettings の DisableScriptModifier を有効にする。
    /// </remarks>
    public sealed class SampleLifetimeScope : LifetimeScope
    {
        [SerializeField]
        [Tooltip("接続先のサーバー。make sandbox-server で起動したサーバーは http://localhost:50051")]
        private string serverAddress = "http://localhost:50051";

        [SerializeField]
        [Tooltip("アプリのバージョン(リリースのたびに増やす整数)。エディタでは editor として送るので、強制アップデートは判定されない")]
        private int appVersion = 1;

        [SerializeField]
        private ScreenNavigator navigator;

        [SerializeField]
        private ErrorDialog errorDialog;

        [SerializeField]
        private TitleScreen titleScreen;

        [SerializeField]
        private GachaScreen gachaScreen;

        private GrpcChannel channel;
        private ServerClockResetter clockResetter;

        /// <inheritdoc />
        protected override void Configure(IContainerBuilder builder)
        {
            var options = new ApiCoreOptions(UnityAppVersion.Create(appVersion));
            // チャンネルは1つを使い回す
            channel = ApiChannel.Create(serverAddress);
            // エディタと開発ビルドでは、通信の Viewer に記録される
            var callInvoker = ApiChannel.CreateCallInvoker(channel);
            // ApiCore と ServerClockResetter に同じ時計を渡す(前面に戻ったら合わせ直す)
            var clock = new ServerClock();

            builder.RegisterInstance(callInvoker).As<CallInvoker>();
            builder.RegisterInstance(clock).As<IServerClock>();
            builder.Register<PlayerRepository>(Lifetime.Singleton);
            builder.Register<BadgeStore>(Lifetime.Singleton);
            builder.Register<NotificationQueue>(Lifetime.Singleton);
            builder.Register(resolver => new ApiCore(
                    options,
                    // アカウント情報の保存先は、サンプルではメモリに置く(起動するたびに新しい端末になる)
                    new DeviceTokenProvider(callInvoker, new InMemoryAccountStore(), options),
                    // 業務エラーの既定の action は、errors.toml から生成した一覧から引く
                    new ErrorPolicyProvider(SandboxErrors.All),
                    new DefaultCommonErrorHandler(errorDialog),
                    new PlayerDiffApplier(resolver.Resolve<PlayerRepository>()),
                    new UnityApiLogger(),
                    clock,
                    navigator,
                    new BadgeApplier(resolver.Resolve<BadgeStore>()),
                    new NotificationApplier(resolver.Resolve<NotificationQueue>())),
                Lifetime.Singleton);
            builder.Register(resolver => new PlayerApi(resolver.Resolve<ApiCore>(), callInvoker), Lifetime.Singleton);
            builder.Register(resolver => new GachaApi(resolver.Resolve<ApiCore>(), callInvoker), Lifetime.Singleton);

            builder.RegisterComponent(navigator);
            builder.RegisterComponent(titleScreen);
            builder.RegisterComponent(gachaScreen);

            builder.RegisterBuildCallback(_ => clockResetter = ServerClockResetter.Create(clock));
        }

        /// <inheritdoc />
        protected override void OnDestroy()
        {
            base.OnDestroy();
            if (clockResetter != null)
            {
                Destroy(clockResetter.gameObject);
            }

            channel?.Dispose();
        }
    }
}
