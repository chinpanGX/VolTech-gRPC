---
title: "認証とアカウント情報"
status: confirmed
area: client
related: [client/structure]
proto: [server/rust/auth/proto/voltech/v1/account_store.proto]
code: [Unity/com.chinpangx.voltech.grpc/Runtime/Client/Auth, Unity/com.chinpangx.voltech.grpc/Runtime/Client/Internal/AesHmacCipher.cs, Unity/com.chinpangx.voltech.grpc/Runtime/Client/Internal/SafeFile.cs, Unity/com.chinpangx.voltech.grpc/Runtime/Unity/UnityAccountStore.cs]
updated: 2026-10-18
---

# 認証とアカウント情報

`VolTech.Client` アセンブリの `Auth/`(名前空間 `VolTech.Client.Auth`。Unity に依存しない .NET のコード)が持つ。`ApiCore` とは、トークンを渡す interface(例: `ITokenProvider`)でつなぐ。保存先のパスとバンドルIDは、Unity 用の層が渡す。

## デバイス登録とトークン

- **デバイス登録とトークンの発行は自動で行う**(`ApiCore` がトークンを求めたときに、Auth が行う)。`device_id` と `secret_key` が無ければデバイス登録し、有効なトークンが無ければ(または有効期限が近ければ。残り5分)発行してから、呼び出されたAPIを送る。アプリは、タイトル画面でサインインを呼ぶだけでよい
- デバイス登録も、トークンの発行も **single-flight** にする(同時に走ると、`device_id` が2つ発行されたり、互いのトークンを無効にし合ったりするため)
- デバイス登録は、アプリが最初にAPIを呼んだ時点で行われる。利用規約への同意などを先に済ませたい場合は、その後で最初のAPIを呼ぶ
- それでも `UNAUTHENTICATED` が返ったら、再認証して**1回だけ**同じRequestIDで送り直す。サーバーが処理を始める前に弾いたものなので、二重実行にならない
- 起動時の流れ: デバイス登録(未登録なら) → トークンの発行 → サインイン →(「プレイヤー未作成」なら)サインアップ

## 保存の仕組み

端末に保存するのは `device_id`、`secret_key`、`player_id`。3つともデバイス登録で揃う。保存の仕組みは基盤が用意し、アプリが差し替えられるようにする。

```csharp
public interface IAccountStore
{
    UniTask<AccountData> LoadAsync();    // 未登録なら null
    UniTask SaveAsync(AccountData data);
    UniTask ClearAsync();
}
```

- **保存先は接続先(サーバーの URL)ごとに分ける**。開発中に接続先を切り替えると、別の環境の DB には保存した `device_id` が無く、トークンを発行できなくなるため。接続先を切り替えたら、その接続先用のアカウント情報を読む(無ければ登録する)
- 非同期にするのは、OSの安全な保存領域を使う実装に差し替えることも考えるため。`ApiCore` は最初の読み込みでメモリに持ち、毎回は読まない
- `player_id` を端末に保存するのは、サインインできないとき(メンテナンス・アカウント停止など)にも問い合わせ用に表示するため。サーバーとずれないように、値はデバイス登録で受け取ったものだけを使う([認証](../api/auth.md))

| タイミング | 書く人 | 内容 |
|---|---|---|
| デバイス登録の後 | Auth | `device_id`、`secret_key`、`player_id` |
| アカウントの引き継ぎ・データ削除 | アプリ | 入れ替える、または消す。その後 `ApiCore` のセッションを破棄する(例: `ResetSession()`)。次の呼び出しで読み直し、トークンを取り直す |

## 保存したアカウント情報が使えないとき

トークンの発行が、保存した `device_id`・`secret_key` で通らなくなることがある(引き継ぎ・削除・DB の復元など。場面は [認証](../api/auth.md))。今の実装では、再認証の後にもう一度弾かれてタイトルに戻り、`ResetSession()` で同じアカウント情報を読み直すので、抜け出せない。次のように扱う。

- トークンの発行が `UNAUTHENTICATED` を返したら、Auth は「保存したアカウント情報が使えない」専用のエラーにする(トークンの取り直しでは解決しないので、`ApiCore` の1回の再認証の対象にしない)
- サーバーが無効にした理由(引き継ぎ済み・削除済みなど)の業務エラーが返ったら、その理由のまま返す
- アプリは、新しく始める(保存したアカウント情報を `ClearAsync` してから `ResetSession()`)か、引き継ぎ・サポートへ進むかを選ばせる。**Auth は自動で登録し直さない**

## 既定の実装(`EncryptedFileAccountStore`)

自作パッケージ Supplement の暗号化ファイル保存(`AesHmacCryptoAlgorithm`・`SafeFile`)を、**パッケージとしては依存せず、コードをコピーして**作り変えた(`Internal/AesHmacCipher.cs`・`Internal/SafeFile.cs`)。本体(Unity に依存しない)に置くので、壊れたファイルの扱いも含めて EditMode テストでテストできる(`EncryptedFileAccountStoreTests`)。

```
[識別子 "VTA1" | バージョン | salt の長さ | salt | 暗号文 | HMAC-SHA256]
                                                  └ AES で暗号化した StoredAccount(Protobuf のバイナリ)
```

Unity では `UnityAccountStore.Create(serverAddress)` で作る。`DeviceTokenProvider` に渡す。

```csharp
var accountStore = UnityAccountStore.Create(serverAddress);   // ApiChannel.Create に渡すものと同じ URL
var tokenProvider = new DeviceTokenProvider(callInvoker, accountStore, options);
```

| 部分 | 基盤での実装 |
|---|---|
| データの形 | 基盤の `.proto` の `StoredAccount`(`voltech/v1/account_store.proto`。通信には使わない)の **Protobuf のバイナリ**にする。項目を後から足しても、古いファイルを読める |
| 暗号化 | AES(CBC)。保存のたびにランダムなsaltを作り、PBKDF2で鍵とIVを作る(Supplementのまま) |
| 改ざん・破損の検出 | **HMAC**を付け(Supplementのフォーマットからバージョンを上げる)、復号する前に確かめる。CBCでは途中が壊れても復号が成功することがあり、`secret_key` だけが黙って壊れ得るため |
| 鍵の元 | パッケージで決めた値とバンドルID(`Application.identifier`。空ならプロダクト名)から作る。アプリは渡さない(`EncryptedFileAccountStore` のコンストラクタで上書きはできる)。鍵の元はファイルに含めない |
| 書き込み | 一時ファイルに書いてから置き換える(Supplementの `SafeFile` のまま) |
| 保存先 | `Application.persistentDataPath/VolTechGrpc/account-<接続先の URL の SHA-256 の先頭 16 文字>.bin`。パッケージ用のフォルダにし、アプリのセーブデータと共有しない。URL の末尾の `/` の有無は同じ接続先として扱う |
| 公開範囲 | 名前空間をパッケージのものに変え、`internal` にする(アプリがSupplementも使っていても名前が衝突しないように) |

- **読み込みのルール**: ファイルが無ければ未登録(`null`)。ファイルがあるのに読めない(HMACが合わない、復号できない、値が空)場合は `CorruptedAccountDataException` にし、**新しいアカウントを勝手に作らない・ファイルも消さない**(元のデータに戻れなくなるため)。問い合わせや引き継ぎに誘導する
  - API の呼び出しの中で起きると、`ApiCore` が予約エラー `ACCOUNT_DATA_CORRUPTED`(既定の action は留まる)にする。送り直しやタイトルに戻っても同じファイルを読むだけで直らないので、留まる。アプリはタイトル画面の最初の API の `handledCodes` に入れて受け取り、問い合わせや引き継ぎ、ユーザーが選んだときだけ `ClearAsync` してから新しく始める、に進ませる
- 鍵の元がアプリの中にある以上、暗号化は難読化の位置づけ(他のアプリからはOSのサンドボックスで守られる)。本当に守りたいアプリは、iOSのKeychainやAndroidのKeystoreを使う実装に差し替える
- PBKDF2は本来は要らないが、コストは無視できるので、動作確認済みのコードを変える量を減らすために残す
- 注意(v1ではドキュメントに書くだけ): 保存したファイルはOSのバックアップに含まれることがある。別の端末に戻すと、2台が同じ `device_id` を持ち、認証のたびに互いのトークンを無効にし合う。バックアップから外すかはアプリに任せる
