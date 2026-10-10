using System;
using System.Collections.Generic;
using Cysharp.Threading.Tasks;
using VolTech.Grpc.Client;

namespace Tests.EditMode.TestDoubles
{
    /// <summary>
    /// 遷移先を記録するナビゲーター。遷移の途中に動かす処理も差し込める。
    /// </summary>
    public sealed class RecordingNavigator : IErrorNavigator
    {
        /// <summary>遷移した先("title"・"home"・"store")。遷移した順に並ぶ。</summary>
        public List<string> Navigated { get; } = new();

        /// <summary>
        /// タイトルへの遷移の途中で動かす処理。null なら何もしない。テストで、遷移先の画面が API を呼ぶのに使う。
        /// </summary>
        public Func<UniTask> OnGoToTitle { get; set; }

        /// <summary>
        /// 記録し、OnGoToTitle があれば動かして終わるのを待つ。
        /// </summary>
        public async UniTask GoToTitleAsync()
        {
            Navigated.Add("title");
            if (OnGoToTitle != null)
            {
                await OnGoToTitle();
            }
        }

        /// <summary>
        /// 記録する。
        /// </summary>
        public UniTask GoToHomeAsync()
        {
            Navigated.Add("home");
            return UniTask.CompletedTask;
        }

        /// <summary>
        /// 記録する。
        /// </summary>
        public UniTask OpenStoreAsync()
        {
            Navigated.Add("store");
            return UniTask.CompletedTask;
        }
    }
}
