using Cysharp.Threading.Tasks;

namespace VolTech.Client.Auth
{
    /// <summary>
    /// メモリに持つだけの IAccountStore。テストと、保存の仕組みができるまでの仮の実装。
    /// </summary>
    public sealed class InMemoryAccountStore : IAccountStore
    {
        private AccountData data;

        /// <summary>
        /// 持っているアカウント情報を返す。
        /// </summary>
        public UniTask<AccountData> LoadAsync()
        {
            return UniTask.FromResult(data);
        }

        /// <summary>
        /// アカウント情報をメモリに持つ。
        /// </summary>
        public UniTask SaveAsync(AccountData data)
        {
            this.data = data;
            return UniTask.CompletedTask;
        }

        /// <summary>
        /// 持っているアカウント情報を捨てる。
        /// </summary>
        public UniTask ClearAsync()
        {
            data = null;
            return UniTask.CompletedTask;
        }
    }
}
