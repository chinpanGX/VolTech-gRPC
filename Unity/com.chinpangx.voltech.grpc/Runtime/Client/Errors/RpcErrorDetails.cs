using System.Linq;
using Google.Rpc;
using Grpc.Core;

namespace VolTech.Client
{
    /// <summary>
    /// サーバーがエラーと一緒に返す詳細(リッチエラーモデル)を読む。
    /// </summary>
    public static class RpcErrorDetails
    {
        /// <summary>サーバーがエラーの詳細を入れるトレーラーのキー。</summary>
        public const string TrailerKey = "grpc-status-details-bin";

        /// <summary>
        /// トレーラーの google.rpc.Status から ErrorInfo を取り出す。無ければ null を返す。
        /// </summary>
        public static ErrorInfo GetErrorInfo(RpcException exception)
        {
            // Grpc.StatusProto は Unity NuGet レジストリに無いので、トレーラーを自分でパースする
            var details = exception.Trailers?.GetValueBytes(TrailerKey);
            if (details == null)
            {
                return null;
            }

            var status = Google.Rpc.Status.Parser.ParseFrom(details);
            return status.Details
                .Where(detail => detail.Is(ErrorInfo.Descriptor))
                .Select(detail => detail.Unpack<ErrorInfo>())
                .FirstOrDefault();
        }
    }
}
