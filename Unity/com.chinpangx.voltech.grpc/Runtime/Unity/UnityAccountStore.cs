using System;
using System.IO;
using System.Security.Cryptography;
using System.Text;
using UnityEngine;
using VolTech.Client.Auth;

namespace VolTech.Client.Unity
{
    /// <summary>
    /// アカウント情報を端末に保存する EncryptedFileAccountStore を、Unity の保存先とバンドルIDから作る。
    /// </summary>
    public static class UnityAccountStore
    {
        // アプリのセーブデータと混ざらないように、パッケージ用のフォルダに置く
        private const string DirectoryName = "VolTechGrpc";

        /// <summary>
        /// 接続先ごとのファイルに保存する EncryptedFileAccountStore を返す。メインスレッドから呼ぶ。
        /// </summary>
        /// <remarks>
        /// 保存先は Application.persistentDataPath の下、鍵の元はバンドルID(Application.identifier)。
        /// 開発中に接続先を切り替えると、別の環境の DB には保存した device_id が無くトークンを発行できなくなるので、接続先ごとにファイルを分ける。
        /// </remarks>
        /// <param name="serverAddress">接続先のサーバーの URL。ApiChannel.Create に渡すものと同じ値にする。</param>
        /// <exception cref="ArgumentException">serverAddress が null か空のとき。</exception>
        public static EncryptedFileAccountStore Create(string serverAddress)
        {
            if (string.IsNullOrEmpty(serverAddress))
            {
                throw new ArgumentException("Server address must not be null or empty.", nameof(serverAddress));
            }

            var filePath = Path.Combine(Application.persistentDataPath, DirectoryName, FileName(serverAddress));
            // エディタでバンドルIDを設定していないと空になるので、そのときはプロダクト名を使う
            var keySource = string.IsNullOrEmpty(Application.identifier) ? Application.productName : Application.identifier;
            return new EncryptedFileAccountStore(filePath, keySource);
        }

        // URL をそのままファイル名にすると使えない文字が入るので、ハッシュの先頭を名前にする。末尾の / の有無は同じ接続先として扱う
        internal static string FileName(string serverAddress)
        {
            using var sha256 = SHA256.Create();
            var hash = sha256.ComputeHash(Encoding.UTF8.GetBytes(serverAddress.TrimEnd('/')));
            var name = new StringBuilder("account-");
            for (var i = 0; i < 8; i++)
            {
                name.Append(hash[i].ToString("x2"));
            }

            return name.Append(".bin").ToString();
        }
    }
}
