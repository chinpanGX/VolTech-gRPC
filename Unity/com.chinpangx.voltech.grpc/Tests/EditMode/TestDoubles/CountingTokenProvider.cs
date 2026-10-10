using System;
using Cysharp.Threading.Tasks;
using VolTech.Grpc.Client;

namespace Tests.EditMode.TestDoubles
{
    /// <summary>
    /// 発行し直すたびに token-1、token-2 … と番号を上げたトークンを返す ITokenProvider。
    /// </summary>
    public sealed class CountingTokenProvider : ITokenProvider
    {
        private int issuedCount;
        private string current;

        /// <summary>GetTokenAsync で投げる例外。null なら投げない。</summary>
        public Exception ThrowOnGet { get; set; }

        /// <summary>Invalidate が呼ばれた回数。</summary>
        public int InvalidateCount { get; private set; }

        /// <summary>Reset が呼ばれた回数。</summary>
        public int ResetCount { get; private set; }

        /// <summary>
        /// 今のトークンを返す。無ければ番号を上げて発行する。
        /// </summary>
        public UniTask<string> GetTokenAsync()
        {
            if (ThrowOnGet != null)
            {
                throw ThrowOnGet;
            }

            if (current == null)
            {
                issuedCount++;
                current = $"token-{issuedCount}";
            }

            return UniTask.FromResult(current);
        }

        /// <summary>
        /// 今のトークンを捨てる。
        /// </summary>
        public void Invalidate(string rejectedToken)
        {
            InvalidateCount++;
            current = null;
        }

        /// <summary>
        /// 今のトークンを捨てる。
        /// </summary>
        public void Reset()
        {
            ResetCount++;
            current = null;
        }
    }
}
