using Cysharp.Threading.Tasks;
using Google.Protobuf;
using Sandbox.V1;
using VolTech.Client;

namespace Sandbox
{
    /// <summary>
    /// レスポンスの player_diff を PlayerRepository に反映する。
    /// </summary>
    public sealed class PlayerDiffApplier : IPlayerDiffApplier
    {
        private readonly PlayerRepository repository;

        /// <summary>
        /// 反映先の Repository を受け取って作る。
        /// </summary>
        public PlayerDiffApplier(PlayerRepository repository)
        {
            this.repository = repository;
        }

        /// <summary>
        /// レスポンスに player_diff があれば Repository に反映する。フィールドが無い・値が入っていなければ何もしない。
        /// </summary>
        public UniTask ApplyAsync(IMessage response)
        {
            // player_diff を持つレスポンスには、protoc-gen-voltech-grpc が IHasPlayerDiff を付けている
            if (response is IHasPlayerDiff { PlayerDiff: { } diff })
            {
                Apply(diff);
            }

            return UniTask.CompletedTask;
        }

        // 差分に入っているものだけを反映する。入っていないもの(変わっていないもの)は触らない
        private void Apply(PlayerDiff diff)
        {
            if (diff.HasGems)
            {
                repository.SetGems(diff.Gems);
            }

            if (diff.Items != null)
            {
                repository.UpsertItems(diff.Items.Upserted);
                repository.RemoveItems(diff.Items.Removed);
            }

            if (diff.HasCounter)
            {
                repository.SetCounter(diff.Counter);
            }
        }
    }
}
