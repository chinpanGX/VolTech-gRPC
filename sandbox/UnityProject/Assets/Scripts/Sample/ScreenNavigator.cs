using Cysharp.Threading.Tasks;
using UnityEngine;
using VolTech.Client;

namespace Sample
{
    /// <summary>
    /// 画面の切り替え。IErrorNavigator の実装例で、エラーの後の「タイトルへ」「ホームへ」「ストアへ」もここを通る。
    /// </summary>
    /// <remarks>
    /// サンプルは1つのシーンで、画面の GameObject の表示を切り替える。ガチャの画面をホームとして扱う。
    /// </remarks>
    public sealed class ScreenNavigator : MonoBehaviour, IErrorNavigator
    {
        [SerializeField]
        private GameObject titleScreen;

        [SerializeField]
        private GameObject homeScreen;

        [SerializeField]
        [Tooltip("強制アップデートのときに開くストアのページ。空なら開かずにログだけ出す")]
        private string storeUrl = "";

        /// <inheritdoc />
        public UniTask GoToTitleAsync()
        {
            Show(titleScreen);
            return UniTask.CompletedTask;
        }

        /// <inheritdoc />
        public UniTask GoToHomeAsync()
        {
            Show(homeScreen);
            return UniTask.CompletedTask;
        }

        /// <inheritdoc />
        public UniTask OpenStoreAsync()
        {
            if (string.IsNullOrEmpty(storeUrl))
            {
                Debug.LogWarning("[Sample] ストアの URL が設定されていない");
            }
            else
            {
                Application.OpenURL(storeUrl);
            }

            return UniTask.CompletedTask;
        }

        private void Start()
        {
            Show(titleScreen);
        }

        private void Show(GameObject screen)
        {
            titleScreen.SetActive(screen == titleScreen);
            homeScreen.SetActive(screen == homeScreen);
        }
    }
}
