using System.Collections.Generic;
using Google.Protobuf;
using Google.Protobuf.WellKnownTypes;
using Google.Rpc;
using Grpc.Core;

namespace VolTech.Client
{
    /// <summary>
    /// 偽のサーバー(FakeCallInvoker に登録する処理)が投げる、サーバーと同じ形のエラーを作る。
    /// </summary>
    public static class FakeRpcErrors
    {
        private const string ErrorDomain = "framework";

        /// <summary>
        /// 業務エラー(FAILED_PRECONDITION + ErrorInfo)を作る。codeName はエラーコードの名前(例: ERROR_CODE_PLAYER_NOT_FOUND)。
        /// </summary>
        /// <param name="metadata">ErrorInfo.metadata に入れる値(例: MAINTENANCE の end_at)。</param>
        public static RpcException FailedPrecondition(
            string codeName,
            string message = "",
            IReadOnlyDictionary<string, string> metadata = null)
        {
            var errorInfo = new ErrorInfo { Reason = codeName, Domain = ErrorDomain };
            if (metadata != null)
            {
                foreach (var pair in metadata)
                {
                    errorInfo.Metadata.Add(pair.Key, pair.Value);
                }
            }

            var status = new Google.Rpc.Status
            {
                Code = (int)StatusCode.FailedPrecondition,
                Message = message,
                Details = { Any.Pack(errorInfo) },
            };
            var trailers = new Metadata { { RpcErrorDetails.TrailerKey, status.ToByteArray() } };
            return new RpcException(new global::Grpc.Core.Status(StatusCode.FailedPrecondition, message), trailers);
        }

        /// <summary>
        /// 詳細の無いエラー(UNAUTHENTICATED・UNAVAILABLE など)を作る。
        /// </summary>
        public static RpcException Status(StatusCode statusCode, string message = "")
        {
            return new RpcException(new global::Grpc.Core.Status(statusCode, message));
        }
    }
}
