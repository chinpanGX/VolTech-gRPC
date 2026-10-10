using Cysharp.Threading.Tasks;
using Google.Protobuf;

namespace VolTech.Grpc.Client
{
    /// <summary>
    /// 成功したレスポンスの badges で、バッジの状態を更新する。アプリが実装する。
    /// </summary>
    /// <remarks>
    /// Badge はアプリの型なので、基盤はレスポンスごと渡す。レスポンスに含まれない種類は「変化なし」なので触らない。
    /// 反映処理でやるのはデータの更新だけにし、ダイアログや画面遷移は書かない。
    /// </remarks>
    public interface IBadgeApplier
    {
        /// <summary>
        /// レスポンスに badges があれば反映する。無ければ何もしない。
        /// </summary>
        public UniTask ApplyAsync(IMessage response);
    }
}
