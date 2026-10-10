using System;
using System.Collections.Generic;
using System.Threading;

namespace VolTech.Client.Tracing
{
    /// <summary>
    /// 通信の記録の置き場所。古いものから順に、最大 Capacity 件を持つ。
    /// </summary>
    /// <remarks>
    /// Interceptor の続きはメインスレッドで動くとは限らないので、追加・置き換えは lock で守る。
    /// 読む側(ウィンドウ)は Version を見て、変わっていたら Snapshot を取り直す。
    /// </remarks>
    public sealed class ApiTraceStore
    {
        /// <summary>持つ件数の上限。超えたら古いものから捨てる。</summary>
        public const int Capacity = 500;

        private readonly object gate = new();
        private readonly List<ApiTrace> traces = new();
        private int version;
        private int lastId;

        /// <summary>このアプリ(エディタ、または実機)の通信の記録。ApiChannel.CreateCallInvoker で作った通信が入る。</summary>
        public static ApiTraceStore Shared { get; } = new();

        /// <summary>記録が足された・置き換えられたときに呼ばれる。呼ばれるスレッドは決まっていない。</summary>
        public event Action<ApiTrace> Changed;

        /// <summary>中身が変わるたびに増える版。</summary>
        public int Version => Volatile.Read(ref version);

        /// <summary>記録するか。false の間は、Interceptor が記録しない(ウィンドウの一時停止)。</summary>
        public bool Recording { get; set; } = true;

        /// <summary>
        /// 新しい記録の連番を返す。
        /// </summary>
        public int NextId()
        {
            return Interlocked.Increment(ref lastId);
        }

        /// <summary>
        /// 新しい記録を末尾に足す。Capacity を超えたら古いものから捨てる。
        /// </summary>
        public void Add(ApiTrace trace)
        {
            lock (gate)
            {
                AddLocked(trace);
                version++;
            }

            Changed?.Invoke(trace);
        }

        /// <summary>
        /// 同じ Id の記録を置き換える。その記録がもう無い(Clear や Capacity 超えで捨てた)ときは何もせず、false を返す。
        /// </summary>
        /// <remarks>
        /// 送信中に消した記録が、応答を受け取ったときに戻ってこないようにする。
        /// </remarks>
        public bool Update(ApiTrace trace)
        {
            lock (gate)
            {
                var index = traces.FindIndex(existing => existing.Id == trace.Id);
                if (index < 0)
                {
                    return false;
                }

                traces[index] = trace;
                version++;
            }

            Changed?.Invoke(trace);
            return true;
        }

        /// <summary>
        /// 同じ Id の記録があれば置き換え、無ければ末尾に足す。実機から受け取った記録を入れるときに使う。
        /// </summary>
        /// <remarks>
        /// 実機は、つながったときに持っている記録をすべて送り直してくるので、エディタの側に無い記録も足す。
        /// </remarks>
        public void Put(ApiTrace trace)
        {
            lock (gate)
            {
                var index = traces.FindIndex(existing => existing.Id == trace.Id);
                if (index >= 0)
                {
                    traces[index] = trace;
                }
                else
                {
                    AddLocked(trace);
                }

                version++;
            }

            Changed?.Invoke(trace);
        }

        /// <summary>
        /// 今の記録を、古い順に返す(複製した一覧なので、後から中身が変わらない)。
        /// </summary>
        public ApiTrace[] Snapshot()
        {
            lock (gate)
            {
                return traces.ToArray();
            }
        }

        /// <summary>
        /// 記録をすべて捨てる。
        /// </summary>
        public void Clear()
        {
            lock (gate)
            {
                traces.Clear();
                version++;
            }
        }

        private void AddLocked(ApiTrace trace)
        {
            traces.Add(trace);
            if (traces.Count > Capacity)
            {
                traces.RemoveAt(0);
            }
        }
    }
}
