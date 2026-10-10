using UnityEngine;
using VolTech.Grpc.Client;

namespace Api.Unity
{
    /// <summary>
    /// 動いているプラットフォームから AppVersion を作る。
    /// </summary>
    public static class UnityAppVersion
    {
        /// <summary>
        /// 今のプラットフォームと、アプリが渡すバージョンの AppVersion を返す。
        /// エディタでは、OS によらず Editor を送る(サーバーは強制アップデートを判定しない)。
        /// サーバーが知らないプラットフォームでは null を返す(ヘッダーを付けない)。
        /// </summary>
        /// <param name="version">リリースのたびに増やす整数。iOS の Build、Android の versionCode と同じ値にする。Unity から実行時に読めないので、アプリが渡す。</param>
        public static AppVersion Create(int version)
        {
            var platform = Application.isEditor ? AppVersion.Editor : PlatformName(Application.platform);
            return platform == null ? null : new AppVersion(platform, version);
        }

        private static string PlatformName(RuntimePlatform platform)
        {
            switch (platform)
            {
                case RuntimePlatform.IPhonePlayer:
                    return AppVersion.Ios;
                case RuntimePlatform.Android:
                    return AppVersion.Android;
                case RuntimePlatform.WindowsPlayer:
                    return AppVersion.Windows;
                default:
                    return null;
            }
        }
    }
}
