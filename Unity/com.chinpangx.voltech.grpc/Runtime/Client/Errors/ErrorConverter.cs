using System;
using System.Collections.Generic;
using Google.Protobuf;
using Grpc.Core;

namespace VolTech.Client
{
    /// <summary>
    /// 受け取ったエラーを、エラーコードと action に変換する。
    /// </summary>
    internal static class ErrorConverter
    {
        private const string ActionMetadataKey = "action";

        /// <summary>
        /// gRPC のエラーをエラーコードに変える。
        /// </summary>
        public static ApiError FromRpcException(RpcException exception, IErrorPolicyProvider policy)
        {
            var message = exception.Status.Detail;
            switch (exception.StatusCode)
            {
                case StatusCode.FailedPrecondition:
                    var errorInfo = TryGetErrorInfo(exception);
                    if (errorInfo != null && policy.TryGetCode(errorInfo.Reason, out var code))
                    {
                        errorInfo.Metadata.TryGetValue(ActionMetadataKey, out var actionOverride);
                        var metadata = new Dictionary<string, string>(errorInfo.Metadata);
                        return Create(code, exception.StatusCode, message, policy, actionOverride, metadata);
                    }

                    return Create(ReservedErrorCodes.UnknownError, exception.StatusCode, message, policy);
                case StatusCode.Internal:
                    return Create(ReservedErrorCodes.InternalError, exception.StatusCode, message, policy);
                case StatusCode.Unavailable:
                case StatusCode.DeadlineExceeded:
                    return Create(ReservedErrorCodes.NetworkError, exception.StatusCode, message, policy);
                default:
                    return Create(ReservedErrorCodes.UnknownError, exception.StatusCode, message, policy);
            }
        }

        /// <summary>
        /// クライアント内部の例外を CLIENT_ERROR に変える。
        /// </summary>
        public static ApiError FromClientException(Exception exception, IErrorPolicyProvider policy)
        {
            return Create(ReservedErrorCodes.ClientError, StatusCode.Unknown, exception.Message, policy);
        }

        // エラーの詳細が壊れていたら(途中の機器が書き換えた、など)、詳細が無いものとして UNKNOWN_ERROR にする
        private static Google.Rpc.ErrorInfo TryGetErrorInfo(RpcException exception)
        {
            try
            {
                return RpcErrorDetails.GetErrorInfo(exception);
            }
            catch (InvalidProtocolBufferException)
            {
                return null;
            }
        }

        private static ApiError Create(
            int code,
            StatusCode statusCode,
            string message,
            IErrorPolicyProvider policy,
            string actionOverride = null,
            IReadOnlyDictionary<string, string> metadata = null)
        {
            var action = ParseAction(actionOverride);
            if (action == ErrorAction.Unspecified)
            {
                action = policy.GetDefaultAction(code);
            }

            if (action == ErrorAction.Unspecified)
            {
                action = ErrorAction.Title;
            }

            return new ApiError(code, policy.GetName(code), action, statusCode, message, metadata);
        }

        // サーバーは metadata["action"] に ErrorAction の値の名前を入れる。知らない値なら既定の action を使う
        private static ErrorAction ParseAction(string name)
        {
            switch (name)
            {
                case "ERROR_ACTION_STAY":
                    return ErrorAction.Stay;
                case "ERROR_ACTION_RETRY":
                    return ErrorAction.Retry;
                case "ERROR_ACTION_HOME":
                    return ErrorAction.Home;
                case "ERROR_ACTION_TITLE":
                    return ErrorAction.Title;
                case "ERROR_ACTION_UPDATE":
                    return ErrorAction.Update;
                default:
                    return ErrorAction.Unspecified;
            }
        }
    }
}
