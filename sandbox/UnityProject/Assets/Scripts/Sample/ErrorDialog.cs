using System;
using Cysharp.Threading.Tasks;
using TMPro;
using UnityEngine;
using VolTech.Client;

namespace Sample
{
    /// <summary>
    /// エラーダイアログ。IErrorDialogView の実装例で、DefaultCommonErrorHandler が出す。
    /// </summary>
    /// <remarks>
    /// ボタンは2つ(主・副)を用意し、組み合わせに合わせて文言と表示を変える。副のボタンを使うのは「再試行 / タイトルへ」だけ。
    /// </remarks>
    public sealed class ErrorDialog : MonoBehaviour, IErrorDialogView
    {
        [SerializeField]
        private GameObject panel;

        [SerializeField]
        private TMP_Text messageText;

        [SerializeField]
        private UnityEngine.UI.Button primaryButton;

        [SerializeField]
        private TMP_Text primaryLabel;

        [SerializeField]
        private UnityEngine.UI.Button secondaryButton;

        [SerializeField]
        private TMP_Text secondaryLabel;

        private UniTaskCompletionSource<ErrorDialogButton> pressed;
        private ErrorDialogButton primaryResult;
        private ErrorDialogButton secondaryResult;

        /// <inheritdoc />
        public async UniTask<ErrorDialogButton> ShowAsync(ApiError error, ErrorDialogButtons buttons)
        {
            messageText.text = ErrorMessages.For(error);
            switch (buttons)
            {
                case ErrorDialogButtons.RetryOrTitle:
                    SetButtons(ErrorDialogButton.Retry, "Retry", ErrorDialogButton.Title, "Title");
                    break;
                case ErrorDialogButtons.Ok:
                    SetButtons(ErrorDialogButton.Ok, "OK");
                    break;
                case ErrorDialogButtons.Home:
                    SetButtons(ErrorDialogButton.Home, "Home");
                    break;
                case ErrorDialogButtons.Store:
                    SetButtons(ErrorDialogButton.Store, "Store");
                    break;
                case ErrorDialogButtons.Title:
                    SetButtons(ErrorDialogButton.Title, "Title");
                    break;
                default:
                    throw new ArgumentOutOfRangeException(nameof(buttons), buttons, null);
            }

            // ApiCore がダイアログを1つずつ出すので、前のダイアログが開いたまま呼ばれることはない
            pressed = new UniTaskCompletionSource<ErrorDialogButton>();
            panel.SetActive(true);
            try
            {
                return await pressed.Task;
            }
            finally
            {
                panel.SetActive(false);
            }
        }

        private void Awake()
        {
            primaryButton.onClick.AddListener(() => pressed?.TrySetResult(primaryResult));
            secondaryButton.onClick.AddListener(() => pressed?.TrySetResult(secondaryResult));
            panel.SetActive(false);
        }

        private void SetButtons(
            ErrorDialogButton primary,
            string primaryText,
            ErrorDialogButton? secondary = null,
            string secondaryText = null)
        {
            primaryResult = primary;
            primaryLabel.text = primaryText;
            secondaryButton.gameObject.SetActive(secondary.HasValue);
            if (secondary.HasValue)
            {
                secondaryResult = secondary.Value;
                secondaryLabel.text = secondaryText;
            }
        }
    }
}
