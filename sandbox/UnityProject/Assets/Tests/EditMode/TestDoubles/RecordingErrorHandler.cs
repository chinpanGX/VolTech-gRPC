using System;
using System.Collections.Generic;
using Cysharp.Threading.Tasks;
using VolTech.Client;

namespace Sandbox.Tests.EditMode.TestDoubles
{
    /// <summary>
    /// 受け取ったエラーを記録し、決めておいた動きを返すエラーハンドラー。
    /// </summary>
    public sealed class RecordingErrorHandler : ICommonErrorHandler
    {
        private readonly Queue<ErrorDecision> decisions = new();

        /// <summary>受け取ったエラー。受け取った順に並ぶ。</summary>
        public List<ApiError> Errors { get; } = new();

        /// <summary>
        /// 呼ばれたときに待つ処理。null なら待たない。テストで、ダイアログが出ている間を作るのに使う。
        /// </summary>
        public Func<UniTask> OnHandle { get; set; }

        /// <summary>
        /// 次に返す動きを足す。足していなければ Stay を返す。
        /// </summary>
        public RecordingErrorHandler ThenReturn(ErrorDecision decision)
        {
            decisions.Enqueue(decision);
            return this;
        }

        /// <summary>
        /// エラーを記録し、足しておいた動きを返す。
        /// </summary>
        public async UniTask<ErrorDecision> HandleAsync(ApiError error)
        {
            Errors.Add(error);
            if (OnHandle != null)
            {
                await OnHandle();
            }

            return decisions.Count > 0 ? decisions.Dequeue() : ErrorDecision.Stay;
        }
    }
}
