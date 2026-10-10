---
title: "サンプルのクライアント(画面)"
status: current
area: client
sections: []
related: [overview, client/api-core, client/error-handling, client/auth-account, server/status]
proto: [proto/app/v1/player.proto, proto/app/v1/gacha.proto]
code: [Unity/UnityProject/Assets/Scripts/Sample]
updated: 2026-10-07
---

# サンプルのクライアント(画面)

基盤の使い方を見せるための、Unity のサンプル画面。基盤の共通処理(自動の認証、エラーダイアログと画面の遷移、player_diff の反映、サーバー時刻)を、ガチャ1つで一通り動かせるようにしている([概要](../overview.md) の 1.3)。

## 画面

| 画面 | やること |
|---|---|
| タイトル | 「Start」で、サインインして所持データを全件取得する。プレイヤーがまだいなければ(`PLAYER_NOT_FOUND`)、サインアップしてからサインインし直す。デバイス登録とトークンの発行は、最初の API を呼ぶときに `ApiCore` が自動で行う |
| ガチャ(ホームを兼ねる) | ジェムの残数・所持品・サーバー時刻を表示する。「Draw」でガチャを1回引き、player_diff で更新された `PlayerRepository` を読み直して表示する。「Free draw」は1日1回の無料ガチャで、まだ引いていなければバッジ(赤い丸)が出る。レアが出たら、通知の待ち行列から取り出して「Rare!」を出す。「Back to title」でセッションを捨ててタイトルに戻る |
| エラーダイアログ | `DefaultCommonErrorHandler` が出す。ジェム不足・無料ガチャの使い切りは「OK」で画面に留まり、メンテナンスは終了予定を出して「Title」でタイトルに戻る |

## 置き場所

- スクリプト: `Unity/UnityProject/Assets/Scripts/Sample/`(asmdef `Sample`)
- シーン: `Unity/UnityProject/Assets/Scenes/Sample.unity`(ビルドするシーンの先頭)

| クラス | 役割 |
|---|---|
| `SampleLifetimeScope` | VContainer で部品を組み立てる。`ApiCore`・`DeviceTokenProvider`・`PlayerApi`・`GachaApi`・`PlayerRepository`・`BadgeStore`・`NotificationQueue`・`ServerClock` を作り、画面に渡す |
| `ScreenNavigator` | `IErrorNavigator` の実装例。画面の GameObject の表示を切り替える |
| `ErrorDialog` | `IErrorDialogView` の実装例 |
| `ErrorMessages` | エラーコードの名前からダイアログの文言を作る。メンテナンスの終了予定は端末のタイムゾーンで出す |
| `TitleScreen`・`GachaScreen` | 画面 |

## 実装で決めたこと

- **UI は uGUI**(Canvas とシーン)。テキストは TextMeshPro
- **シーンは1つ**で、タイトルとガチャの GameObject の表示を切り替える。エラーの後の「タイトルへ」「ホームへ」も、この切り替えにする。ガチャの画面をホームとして扱う(画面を増やさずに、基盤の動きをすべて試せる)
- ガチャの画面は、シーンでは隠しておく。`LifetimeScope` が部品を組み立てる前に `OnEnable` が呼ばれると、渡されていない部品を読んでしまうため
- **文言は英語**にする。日本語のフォントを入れていないため。実際のアプリでは、エラーコードの名前から作った文言キー([エラー](../api/errors.md))でローカライズの表を引く
- **アカウント情報の保存先はメモリ**(`InMemoryAccountStore`)。起動するたびに新しい端末として登録する。暗号化して保存する実装([認証とアカウント情報](auth-account.md))は、まだ無い
- エディタでは、アプリのバージョンは `editor` として送るので、強制アップデートは判定されない([サーバーの状態](../server/status.md))
- `ServerClock` は `SampleLifetimeScope` が作り、`ApiCore` と `ServerClockResetter` に同じものを渡す([ApiCore の共通処理](api-core.md))。ガチャの画面は `IsSynchronized` を確かめてからサーバー時刻を表示する
- 強制アップデートで開くストアの URL は、`ScreenNavigator` に設定する。空なら開かずにログだけ出す
- 入力は Input System だけなので、EventSystem には `InputSystemUIInputModule` を付ける
- 通知は、ガチャの結果を出した後に取り出して出す(反映処理の中では出さない。[共通レスポンス項目の反映](response-items.md))

## 作るときに気をつけること

- **VContainer は、名前が `LifetimeScope.cs` で終わる新しいスクリプトを、ひな形で上書きする**(`ScriptTemplateModifier`)。Unity がそのスクリプトの meta を作るときに動くので、エディタの外で書いたファイルも上書きされる。meta ができてから中身を書き直すか、`VContainerSettings` の `DisableScriptModifier` を有効にする
- `[SerializeField]` のフィールドは readonly にできないので、`Sample` フォルダでは IDE0044 を切っている(`Unity/UnityProject/.editorconfig`)

## 採らなかった案

| 項目 | 採らなかった案 | 理由 |
|---|---|---|
| UI | UI Toolkit(UXML と USS) | 差分は読みやすいが、実務で広く使われている uGUI にそろえた |
| 画面 | タイトル・ホーム・ガチャの3画面 | 「ホームへ」の遷移は分かりやすくなるが、画面が増えるだけで試せる基盤の動きは変わらない |
| 画面の切り替え | 画面ごとにシーンを分ける | サンプルでは、シーンの読み込みや部品の受け渡しが増えるだけ |
