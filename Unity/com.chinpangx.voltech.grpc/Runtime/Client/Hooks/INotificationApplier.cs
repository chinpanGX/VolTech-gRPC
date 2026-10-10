using Cysharp.Threading.Tasks;
using Google.Protobuf;

namespace VolTech.Client
{
    /// <summary>
    /// 成功したレスポンスの notifications を、待ち行列に積む。アプリが実装する。
    /// </summary>
    /// <remarks>
    /// Notification はアプリの型なので、基盤はレスポンスごと渡す。ここでは積むだけにし、通知を出すタイミングは画面が決める
    /// (反映処理の中で出すと、ガチャの演出の途中に割り込む)。
    /// </remarks>
    public interface INotificationApplier
    {
        /// <summary>
        /// レスポンスに notifications があれば積む。無ければ何もしない。
        /// </summary>
        public UniTask ApplyAsync(IMessage response);
    }
}
