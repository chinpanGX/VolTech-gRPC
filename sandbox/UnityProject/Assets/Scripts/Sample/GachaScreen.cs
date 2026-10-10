using System.Linq;
using Cysharp.Threading.Tasks;
using Sandbox;
using Sandbox.V1;
using TMPro;
using UnityEngine;
using VContainer;
using VolTech.Client;

namespace Sample
{
    /// <summary>
    /// ガチャの画面(サンプルではホームを兼ねる)。ガチャを引くと、player_diff でジェムと所持品が更新される。
    /// </summary>
    /// <remarks>
    /// 共通レスポンス項目は、DrawAsync が返る前に ApiCore が反映し終えている。この画面は、返った後に
    /// PlayerRepository・BadgeStore を読み直して表示し、NotificationQueue から通知を取り出して出すだけで、レスポンスの中身は読まない。
    /// ジェム不足・無料ガチャの使い切り(action が Stay)は、共通エラーハンドラーがダイアログを出して画面に留まる。
    /// </remarks>
    public sealed class GachaScreen : MonoBehaviour
    {
        [SerializeField]
        private TMP_Text gemsText;

        [SerializeField]
        private TMP_Text itemsText;

        [SerializeField]
        private TMP_Text resultText;

        [SerializeField]
        private TMP_Text serverTimeText;

        [SerializeField]
        private UnityEngine.UI.Button drawButton;

        [SerializeField]
        private UnityEngine.UI.Button freeDrawButton;

        [SerializeField]
        [Tooltip("無料ガチャのバッジ。今日の無料ガチャをまだ引いていなければ出す")]
        private GameObject freeDrawBadge;

        [SerializeField]
        private UnityEngine.UI.Button titleButton;

        private GachaApi gachaApi;
        private ApiCore core;
        private PlayerRepository repository;
        private BadgeStore badges;
        private NotificationQueue notifications;
        private IServerClock clock;
        private ScreenNavigator navigator;

        /// <summary>
        /// 使う部品を受け取る(VContainer が呼ぶ)。
        /// </summary>
        [Inject]
        public void Construct(
            GachaApi gachaApi,
            ApiCore core,
            PlayerRepository repository,
            BadgeStore badges,
            NotificationQueue notifications,
            IServerClock clock,
            ScreenNavigator navigator)
        {
            this.gachaApi = gachaApi;
            this.core = core;
            this.repository = repository;
            this.badges = badges;
            this.notifications = notifications;
            this.clock = clock;
            this.navigator = navigator;
        }

        private void Awake()
        {
            drawButton.onClick.AddListener(() => DrawAsync(free: false).Forget());
            freeDrawButton.onClick.AddListener(() => DrawAsync(free: true).Forget());
            titleButton.onClick.AddListener(() => ReturnToTitleAsync().Forget());
        }

        private void OnEnable()
        {
            SetDrawButtonsInteractable(true);
            resultText.text = "";
            Refresh();
        }

        private void Update()
        {
            // 前面に戻った直後など、サーバー時刻を受け取り直すまでは表示しない
            serverTimeText.text = clock.IsSynchronized
                ? $"Server time: {clock.Now.ToLocalTime():yyyy-MM-dd HH:mm:ss}"
                : "Server time: --";
        }

        private async UniTaskVoid DrawAsync(bool free)
        {
            SetDrawButtonsInteractable(false);
            try
            {
                var drawn = await gachaApi.DrawAsync(new DrawRequest { Free = free });
                if (drawn.IsSuccess)
                {
                    resultText.text = $"Got {ItemName(drawn.Value.Item.ItemId)} (now x{drawn.Value.Item.Count})";
                    // 演出(ここでは結果の表示)の後に、積まれた通知を出す
                    ShowNotifications();
                }
            }
            finally
            {
                SetDrawButtonsInteractable(true);
                Refresh();
            }
        }

        private void ShowNotifications()
        {
            while (notifications.TryDequeue(out var notification))
            {
                switch (notification.EventCase)
                {
                    case Notification.EventOneofCase.RareItemObtained:
                        resultText.text += $"\nRare! {ItemName(notification.RareItemObtained.ItemId)}";
                        break;
                }
            }
        }

        private UniTask ReturnToTitleAsync()
        {
            // タイトルに戻るときはセッションを捨てる(順番待ちの通信も止まる)。次はサインインからやり直す
            core.ResetSession();
            return navigator.GoToTitleAsync();
        }

        private void SetDrawButtonsInteractable(bool interactable)
        {
            drawButton.interactable = interactable;
            freeDrawButton.interactable = interactable;
        }

        private void Refresh()
        {
            gemsText.text = $"Gems: {repository.Gems}";
            itemsText.text = repository.Items.Count == 0
                ? "Items: (none)"
                : "Items:\n" + string.Join("\n", repository.Items
                    .OrderBy(item => item.Key)
                    .Select(item => $"  {ItemName(item.Key)} x{item.Value}"));
            freeDrawBadge.SetActive(badges.IsVisible(BadgeType.FreeGacha));
        }

        // サンプルの排出物はマスタを持たず、ID だけで表す(sandbox/server/gacha/src/model の ITEMS)
        private static string ItemName(int itemId)
        {
            return $"Item #{itemId}";
        }
    }
}
