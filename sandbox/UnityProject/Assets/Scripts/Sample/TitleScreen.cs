using Cysharp.Threading.Tasks;
using Sandbox;
using Sandbox.V1;
using TMPro;
using UnityEngine;
using VContainer;
using VolTech.Client;
using VolTech.Client.Auth;

namespace Sample
{
    /// <summary>
    /// タイトル画面。端末に保存したアカウント情報の状態で、サインアップかサインインに進み、所持データを全件取得してホームへ進む。
    /// </summary>
    /// <remarks>
    /// デバイス登録とトークンの発行は、最初の API を呼ぶときに ApiCore が自動で行う。
    /// メンテナンス・強制アップデートなどは、共通エラーハンドラーがダイアログを出すので、この画面では扱わない。
    /// アカウント情報の読み書きの失敗は通信のエラーではないので、共通エラーハンドラーは扱わず、この画面で専用の案内を出す。
    /// データ削除のボタンで、保存したアカウント情報を消して、新しいプレイヤーで始め直せる(開発中に新しいプレイヤーを作るときにも使う)。
    /// </remarks>
    public sealed class TitleScreen : MonoBehaviour
    {
        private const string TapToStart = "Tap to start";
        private const string CorruptedMessage = "Saved account data is unreadable. Delete data to start as a new player";

        [SerializeField]
        private UnityEngine.UI.Button startButton;

        [SerializeField]
        private UnityEngine.UI.Button deleteButton;

        [SerializeField]
        private TMP_Text statusText;

        private PlayerApi playerApi;
        private PlayerRepository repository;
        private ScreenNavigator navigator;
        private DeviceTokenProvider tokenProvider;
        private IAccountStore accountStore;
        private ApiCore core;

        /// <summary>
        /// 使う部品を受け取る(VContainer が呼ぶ)。
        /// </summary>
        [Inject]
        public void Construct(
            PlayerApi playerApi,
            PlayerRepository repository,
            ScreenNavigator navigator,
            DeviceTokenProvider tokenProvider,
            IAccountStore accountStore,
            ApiCore core)
        {
            this.playerApi = playerApi;
            this.repository = repository;
            this.navigator = navigator;
            this.tokenProvider = tokenProvider;
            this.accountStore = accountStore;
            this.core = core;
        }

        private void Awake()
        {
            startButton.onClick.AddListener(() => StartAsync().Forget());
            deleteButton.onClick.AddListener(() => DeleteDataAsync().Forget());
        }

        private void OnEnable()
        {
            SetButtonsInteractable(true);
            statusText.text = TapToStart;
        }

        private async UniTaskVoid StartAsync()
        {
            SetButtonsInteractable(false);
            // 失敗したときの案内。ホームへ進んだときは画面が隠れているので関係ない
            var message = TapToStart;
            try
            {
                message = await SignInOrSignUpAsync();
                if (message == null)
                {
                    await navigator.GoToHomeAsync();
                    message = TapToStart;
                }
            }
            finally
            {
                // 失敗したら、もう一度押せるようにする
                SetButtonsInteractable(true);
                statusText.text = message;
            }
        }

        // 保存したアカウント情報を消し、覚えているアカウント情報も捨てる。次の Start で新しいプレイヤーとしてサインアップする。
        // サーバーのプレイヤーは消えないので、アプリでは確認のダイアログを出し、問い合わせや引き継ぎの案内と一緒にする
        private async UniTaskVoid DeleteDataAsync()
        {
            SetButtonsInteractable(false);
            var message = TapToStart;
            try
            {
                await accountStore.ClearAsync();
                core.ResetSession();
                message = "Data deleted. Tap to start as a new player";
            }
            finally
            {
                SetButtonsInteractable(true);
                statusText.text = message;
            }
        }

        private void SetButtonsInteractable(bool interactable)
        {
            startButton.interactable = interactable;
            deleteButton.interactable = interactable;
        }

        // 保存したアカウント情報の状態で、サインアップかサインインに進む。ホームへ進めたら null、進めなければ案内を返す。
        // 通信の失敗は、共通エラーハンドラーがダイアログを出し終えているので、最初の案内に戻すだけでよい
        private async UniTask<string> SignInOrSignUpAsync()
        {
            switch (await tokenProvider.GetAccountStatusAsync())
            {
                case AccountStatus.NotRegistered:
                    // サインアップの最初の API で、デバイス登録と保存が自動で行われる
                    statusText.text = "Creating a player...";
                    var signedUp = await playerApi.SignUpAsync(new SignUpRequest());
                    if (!signedUp.IsSuccess)
                    {
                        return FailureMessage(signedUp.Error);
                    }

                    break;
                case AccountStatus.Corrupted:
                    return CorruptedMessage;
            }

            statusText.text = "Signing in...";
            // デバイス登録の後、サインアップの前に止まっていたら、プレイヤーがまだいないので、サインアップしてからサインインし直す
            var signedIn = await playerApi.SignInAsync(new SignInRequest(), new[] { SandboxErrors.PlayerNotFound });
            if (!signedIn.IsSuccess && signedIn.Error.Code == SandboxErrors.PlayerNotFound)
            {
                statusText.text = "Creating a player...";
                var signedUp = await playerApi.SignUpAsync(new SignUpRequest());
                if (!signedUp.IsSuccess)
                {
                    return FailureMessage(signedUp.Error);
                }

                signedIn = await playerApi.SignInAsync(new SignInRequest());
            }

            if (!signedIn.IsSuccess)
            {
                return FailureMessage(signedIn.Error);
            }

            // サインインの全件で置き換える(全件に無いものを残さないように、先に消す)
            repository.SetGems(signedIn.Value.Player.Gems);
            repository.SetCounter(signedIn.Value.Player.Counter);
            repository.ClearItems();
            repository.UpsertItems(signedIn.Value.Items);
            return null;
        }

        // アカウント情報の読み書きの失敗は、共通エラーハンドラーを通らずに返るので、この画面で案内する
        private string FailureMessage(ApiError error)
        {
            if (error.Code == ReservedErrors.AccountDataCorrupted)
            {
                return CorruptedMessage;
            }

            if (error.Code == ReservedErrors.AccountSaveFailed)
            {
                return "Could not save the account on this device. Free up space and tap to retry";
            }

            return TapToStart;
        }
    }
}
