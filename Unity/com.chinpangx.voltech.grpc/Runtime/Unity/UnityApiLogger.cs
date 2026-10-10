using System;
using UnityEngine;
using VolTech.Client;

namespace VolTech.Client.Unity
{
    /// <summary>
    /// 通信のログを Unity の Console に出す。
    /// </summary>
    public sealed class UnityApiLogger : IApiLogger
    {
        /// <summary>
        /// Debug.Log で出す。
        /// </summary>
        public void Log(string message)
        {
            Debug.Log(message);
        }

        /// <summary>
        /// メッセージと例外を Debug.LogException で出す。
        /// </summary>
        public void LogException(string message, Exception exception)
        {
            Debug.LogError(message);
            Debug.LogException(exception);
        }
    }
}
