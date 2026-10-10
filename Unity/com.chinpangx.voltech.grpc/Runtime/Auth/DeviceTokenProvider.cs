using System;
using System.Threading;
using Cysharp.Threading.Tasks;
using Grpc.Core;
using VolTech.Client;
using VolTech.V1;

namespace VolTech.Client.Auth
{
    /// <summary>
    /// デバイス認証でアクセストークンを渡す。未登録ならデバイス登録し、トークンが無いか期限が近ければ発行し直す。
    /// </summary>
    public sealed class DeviceTokenProvider : ITokenProvider
    {
        // 期限ちょうどまで使うと、送っている間に切れることがあるので、早めに取り直す
        private static readonly TimeSpan refreshMargin = TimeSpan.FromMinutes(5);

        private readonly DeviceService.DeviceServiceClient client;
        private readonly IAccountStore accountStore;
        private readonly ApiCoreOptions options;
        private readonly Func<DateTime> utcNow;

        // デバイス登録もトークンの発行も、同時に走ると device_id が2つ発行されたり、互いのトークンを無効にし合ったりするので1つずつ行う
        private readonly SemaphoreSlim gate = new(1, 1);

        private AccountData account;
        private string accessToken;
        private DateTime expiresAt;

        /// <summary>
        /// DeviceService を呼ぶ CallInvoker と、アカウント情報の保存先を受け取って作る。
        /// </summary>
        /// <param name="utcNow">今の時刻を返す処理。テストで差し替える。省略すると DateTime.UtcNow。</param>
        public DeviceTokenProvider(
            CallInvoker callInvoker,
            IAccountStore accountStore,
            ApiCoreOptions options,
            Func<DateTime> utcNow = null)
        {
            client = new DeviceService.DeviceServiceClient(callInvoker);
            this.accountStore = accountStore;
            this.options = options;
            this.utcNow = utcNow ?? (() => DateTime.UtcNow);
        }

        /// <summary>
        /// 有効なアクセストークンを返す。無ければ、デバイス登録やトークンの発行をしてから返す。
        /// </summary>
        public async UniTask<string> GetTokenAsync()
        {
            await gate.WaitAsync();
            try
            {
                if (accessToken != null && utcNow() < expiresAt - refreshMargin)
                {
                    return accessToken;
                }

                account ??= await accountStore.LoadAsync();
                if (account == null)
                {
                    account = await RegisterAsync();
                }

                var issued = await client.IssueTokenAsync(
                    new IssueTokenRequest { DeviceId = account.DeviceId, SecretKey = account.SecretKey },
                    Headers(),
                    Deadline());
                accessToken = issued.AccessToken;
                expiresAt = issued.ExpiresAt.ToDateTime();
                return accessToken;
            }
            finally
            {
                gate.Release();
            }
        }

        /// <summary>
        /// 弾かれたトークンが今のトークンなら捨て、次の GetTokenAsync で発行し直させる。
        /// </summary>
        public void Invalidate(string rejectedToken)
        {
            // 別の呼び出しがすでに発行し直していたら、新しいトークンは捨てない
            if (rejectedToken == null || rejectedToken == accessToken)
            {
                accessToken = null;
            }
        }

        /// <summary>
        /// 覚えているトークンとアカウント情報を捨てる。次の GetTokenAsync で保存先から読み直す。
        /// </summary>
        public void Reset()
        {
            accessToken = null;
            account = null;
        }

        private async UniTask<AccountData> RegisterAsync()
        {
            // サーバーは 16〜128 文字の表示できる ASCII 文字を受け付ける(docs/wiki/api/device-service.md)
            var secretKey = Guid.NewGuid().ToString();
            var registered = await client.RegisterAsync(
                new RegisterRequest { SecretKey = secretKey },
                Headers(),
                Deadline());
            // player_id もデバイス登録で発行されるので、ここで3つとも保存する
            var registeredAccount = new AccountData(registered.DeviceId, secretKey, registered.PlayerId);
            await accountStore.SaveAsync(registeredAccount);
            return registeredAccount;
        }

        // タイトル画面で最初に呼ぶので、ここで強制アップデートを判定してもらえるように、プラットフォームとバージョンを付ける
        private Metadata Headers()
        {
            var headers = new Metadata();
            options.AppVersion?.AddHeadersTo(headers);
            return headers;
        }

        private DateTime Deadline()
        {
            return DateTime.UtcNow.Add(options.Deadline);
        }
    }
}
