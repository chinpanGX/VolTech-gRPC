using Cysharp.Threading.Tasks;
using Google.Protobuf;

namespace VolTech.Grpc.Client
{
    /// <summary>
    /// 成功したレスポンスの player_diff を、アプリのデータ(Repository)に反映する。アプリが実装する。
    /// </summary>
    /// <remarks>
    /// PlayerDiff はアプリの型なので、基盤はレスポンスごと渡す。反映処理でやるのはデータの更新だけにし、ダイアログや画面遷移は書かない。
    /// </remarks>
    public interface IPlayerDiffApplier
    {
        /// <summary>
        /// レスポンスに player_diff があれば反映する。無ければ何もしない。
        /// </summary>
        public UniTask ApplyAsync(IMessage response);
    }
}
