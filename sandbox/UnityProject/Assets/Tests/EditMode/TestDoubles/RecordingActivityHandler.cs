using System;
using System.Collections.Generic;
using VolTech.Client;

namespace Sandbox.Tests.EditMode.TestDoubles
{
    /// <summary>
    /// 進行中の呼び出しの開始・終了を記録するハンドラー。例外を投げさせることもできる。
    /// </summary>
    public sealed class RecordingActivityHandler : IApiActivityHandler
    {
        private readonly string name;
        private readonly List<string> events;

        /// <summary>
        /// 名前と、呼ばれた順を記録する一覧(ほかの RecordingActivityHandler と共有する)を受け取って作る。
        /// </summary>
        public RecordingActivityHandler(string name, List<string> events)
        {
            this.name = name;
            this.events = events;
        }

        /// <summary>OnActivityStarted・OnActivityEnded で投げる例外。null なら投げない。</summary>
        public Exception ThrowOnNotify { get; set; }

        /// <summary>
        /// 「名前 started」を記録する。ThrowOnNotify があれば投げる。
        /// </summary>
        public void OnActivityStarted()
        {
            Record("started");
        }

        /// <summary>
        /// 「名前 ended」を記録する。ThrowOnNotify があれば投げる。
        /// </summary>
        public void OnActivityEnded()
        {
            Record("ended");
        }

        private void Record(string kind)
        {
            events.Add($"{name} {kind}");
            if (ThrowOnNotify != null)
            {
                throw ThrowOnNotify;
            }
        }
    }
}
