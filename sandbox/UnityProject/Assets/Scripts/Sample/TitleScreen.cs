using Cysharp.Threading.Tasks;
using Sandbox;
using Sandbox.V1;
using TMPro;
using UnityEngine;
using VContainer;

namespace Sample
{
    /// <summary>
    /// タイトル画面。サインインで所持データを全件取得し、プレイヤーがまだいなければサインアップしてからホームへ進む。
    /// </summary>
    /// <remarks>
    /// デバイス登録とトークンの発行は、最初の API を呼ぶときに ApiCore が自動で行う。
    /// メンテナンス・強制アップデートなどは、共通エラーハンドラーがダイアログを出すので、この画面では扱わない。
    /// </remarks>
    public sealed class TitleScreen : MonoBehaviour
    {
        [SerializeField]
        private UnityEngine.UI.Button startButton;

        [SerializeField]
        private TMP_Text statusText;

        private PlayerApi playerApi;
        private PlayerRepository repository;
        private ScreenNavigator navigator;

        /// <summary>
        /// 使う部品を受け取る(VContainer が呼ぶ)。
        /// </summary>
        [Inject]
        public void Construct(PlayerApi playerApi, PlayerRepository repository, ScreenNavigator navigator)
        {
            this.playerApi = playerApi;
            this.repository = repository;
            this.navigator = navigator;
        }

        private void Awake()
        {
            startButton.onClick.AddListener(() => StartAsync().Forget());
        }

        private void OnEnable()
        {
            startButton.interactable = true;
            statusText.text = "Tap to start";
        }

        private async UniTaskVoid StartAsync()
        {
            startButton.interactable = false;
            statusText.text = "Signing in...";
            try
            {
                if (await SignInAsync())
                {
                    await navigator.GoToHomeAsync();
                }
            }
            finally
            {
                // 失敗したら、もう一度押せるようにする(ホームへ進んだときは画面が隠れているので関係ない)
                startButton.interactable = true;
                statusText.text = "Tap to start";
            }
        }

        // サインインして全件で置き換える。プレイヤーがまだいなければ、サインアップしてからサインインし直す。
        // 失敗したときは、共通エラーハンドラーがダイアログを出し終えているので、false を返すだけでよい
        private async UniTask<bool> SignInAsync()
        {
            var signedIn = await playerApi.SignInAsync(new SignInRequest(), new[] { SandboxErrors.PlayerNotFound });
            if (!signedIn.IsSuccess && signedIn.Error.Code == SandboxErrors.PlayerNotFound)
            {
                statusText.text = "Creating a player...";
                var signedUp = await playerApi.SignUpAsync(new SignUpRequest());
                if (!signedUp.IsSuccess)
                {
                    return false;
                }

                signedIn = await playerApi.SignInAsync(new SignInRequest());
            }

            if (!signedIn.IsSuccess)
            {
                return false;
            }

            // サインインの全件で置き換える(全件に無いものを残さないように、先に消す)
            repository.SetGems(signedIn.Value.Player.Gems);
            repository.SetCounter(signedIn.Value.Player.Counter);
            repository.ClearItems();
            repository.UpsertItems(signedIn.Value.Items);
            return true;
        }
    }
}
