using System;
using System.Collections.Generic;
using Cysharp.Threading.Tasks;
using Google.Protobuf;
using VolTech.Client;

namespace Sandbox.Tests.EditMode.TestDoubles
{
    /// <summary>
    /// 渡されたレスポンスを記録する反映処理(player_diff・badges・notifications のどれにも使える)。例外を投げさせることもできる。
    /// </summary>
    public sealed class RecordingApplier : IPlayerDiffApplier, IBadgeApplier, INotificationApplier
    {
        private readonly string name;
        private readonly List<string> order;

        /// <summary>
        /// 名前と、呼ばれた順を記録する一覧(ほかの RecordingApplier と共有する)を受け取って作る。
        /// </summary>
        public RecordingApplier(string name, List<string> order)
        {
            this.name = name;
            this.order = order;
        }

        /// <summary>渡されたレスポンス。渡された順に並ぶ。</summary>
        public List<IMessage> Applied { get; } = new();

        /// <summary>ApplyAsync で投げる例外。null なら投げない。</summary>
        public Exception ThrowOnApply { get; set; }

        /// <summary>
        /// レスポンスと、呼ばれた順を記録する。ThrowOnApply があれば投げる。
        /// </summary>
        public UniTask ApplyAsync(IMessage response)
        {
            Applied.Add(response);
            order.Add(name);
            if (ThrowOnApply != null)
            {
                throw ThrowOnApply;
            }

            return UniTask.CompletedTask;
        }
    }
}
