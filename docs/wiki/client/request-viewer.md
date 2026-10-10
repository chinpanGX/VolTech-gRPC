---
title: "通信の Viewer(エディタ拡張)"
status: current
area: client
related: [client/api-core, client/structure, client/connection, ops/debugging, api/metadata, api/errors]
proto: []
code: [Unity/com.chinpangx.voltech.grpc/Runtime/Tracing, Unity/com.chinpangx.voltech.grpc/Editor, Unity/com.chinpangx.voltech.grpc/Runtime/Unity/ApiChannel.cs]
updated: 2026-10-11
---

# 通信の Viewer(エディタ拡張)

Unity エディタで、API のリクエストとレスポンスを1件ずつ見るためのウィンドウ。Console のログ(`UnityApiLogger`)は流れて読みにくく、ヘッダーやエラーの詳細(`ErrorInfo`)も見えないので、通信ごとにヘッダーと Body を分けて見られるようにする。

エディタの Play で動かした通信に加えて、**USB でつないだ実機(開発ビルド)の通信も、同じウィンドウで見られる**(下の「実機の通信を見る」)。

## やりたいこと

- 通信の一覧から1件を選ぶと、リクエストとレスポンスの**ヘッダー**と **Body** を分けて見られる
- ヘッダーは**項目ごとに1行**(名前と値の Label)で出す
- Body は **JSON** で出し、**そのままコピーして、grpcurl や Postman に貼り付けて使える**
- エラーのときは、ステータスと `ErrorInfo`(reason・metadata)も読める形で出す
- UI は **UI Toolkit** で、USS・UXML は用意せず、C# のコードで Element を組み立てる
- **見るだけにする**。Viewer からリクエストを送り直したり、書き換えて送ったりはしない。もう一度送りたいときは、アプリの操作(エラーダイアログの「再試行」、ボタンをもう一度押すなど)で送る。`ApiCore` を通るので、トークン・順番待ち・同じ RequestID での再送が、実際のアプリと同じに動く

## 記録のしかた

**gRPC の Interceptor で記録する**(`ApiTraceInterceptor`)。`ApiCore` の中では記録しない。

- トークンの発行(`DeviceTokenProvider`)は `ApiCore` を通らない。メンテナンス・強制アップデートは、タイトル画面で最初に呼ぶトークンの発行で返ることが多いので、`ApiCore` で記録すると、いちばん見たい通信が見えない
- Interceptor なら、実際に送ったヘッダー(`authorization`・`x-request-id`・`x-app-*`・`x-game-date`)と、受け取ったヘッダー・トレーラーが、そのまま見える
- [ApiCore の共通処理](api-core.md) では「v1 では Interceptor を使わない」としたが、それは共通処理(再送・再認証など)の話で、「ログや計測を gRPC の層で取りたくなったら、そのときに足す」としていた。Viewer はこれに当たる。Interceptor は記録するだけで、リクエストもレスポンスも変えない
- `ApiCore` の再送(UNAUTHENTICATED の後の送り直し、エラーハンドラーの再試行)は、送った回数だけ別の行になる。同じ RequestID(`x-request-id`)で並ぶので、再送だと分かる

Interceptor は、`AsyncUnaryCall` を同じ形で包み直し、レスポンスを待ってから記録して、結果(例外を含む)をそのまま返す。例外は包まずに投げ直す(`AggregateException` にしない)。

**記録の処理で起きた例外は、通信に混ぜない**。JSON への変換や、記録の置き場所の購読者が例外を投げても、通信は成功・失敗をそのまま返し、記録できなかったことを警告のログ(`[ApiTrace]`)に出す。送る前の記録で失敗したときは、その通信は記録せずに送る。開発用のツールのせいで、エディタと開発ビルドだけ通信が失敗する、ということを起こさないため。

### 記録する中身(`ApiTrace`)

| 項目 | 中身 |
|---|---|
| 識別 | 連番、RPC のフルパス(`/app.v1.GachaService/Draw`)、開始時刻(端末の時刻)、所要時間、deadline |
| リクエスト | ヘッダー(順番どおり)、Body(JSON) |
| レスポンス | ヘッダー、Body(JSON。エラーのときは無し)、トレーラー |
| 結果 | 状態(送信中 / 成功 / エラー)、gRPC のステータスコードと detail、`ErrorInfo`(`grpc-status-details-bin` を読んだもの: reason・domain・metadata) |

- Body は `JsonFormatter`(`WithFormatDefaultValues(true)`・`WithIndentation()`)で、既定値のフィールドも含めて整形する。値が既定値でも、送った・受け取ったことが分かるようにするため。protobuf の標準の JSON 形式なので、grpcurl の `-d` と Postman にそのまま貼れる
- `-bin` で終わるヘッダー・トレーラー(バイナリ)は、バイト数と base64 で出す。`grpc-status-details-bin` は `ErrorInfo` として別に出す
- 記録はリクエストを送るときに作り(状態は「送信中」)、レスポンスかエラーを受け取ったら埋める。応答を待っている通信も一覧に出る

### 記録の置き場所(`ApiTraceStore`)

- エディタに1つ(`ApiTraceStore.Shared`)。新しい順に最大 500 件を持ち、超えたら古いものから捨てる
- 送るときの記録は `Add`(末尾に足す)、結果は `Update`(同じ Id の記録があるときだけ置き換える)で入れる。送信中に Clear したり、500 件を超えて捨てたりした記録が、応答を受け取ったときに戻ってこないようにする。実機から受け取った記録は `Put`(無ければ足す)で入れる
- Interceptor の続きはメインスレッドで動くとは限らないので、追加・更新は lock で守り、変わるたびに版(`Version`)を増やす。ウィンドウは定期的(200ms)に版を見て、変わっていたら描き直す(別スレッドから UI に触らない)
- 記録はメモリにだけ持つ。Play を止めても消えないので、止めてから見返せる。エディタを閉じると消える
- Play を始める前(`ExitingEditMode`)に消す(Clear on Play。既定はオン)。ドメインのリロードを切っていても、前の Play の記録と混ざらない。Play を始めた後(`EnteredPlayMode`)に消すと、最初のシーンの初期化で送った通信の記録まで消えるので、その前に消す。ウィンドウを閉じていても効くように、消す処理は `ApiTraceReceiver`(`[InitializeOnLoadMethod]`)が持つ

### 組み込み方

- `ApiChannel.CreateCallInvoker(channel)` で CallInvoker を作る。`ApiChannel.AddInterceptor` で足された Interceptor を付ける。付ける Interceptor は呼ぶたびに決めるので、CallInvoker を作った後に足された Interceptor も付く(登録の順番に頼らない)。サンプル(`SampleLifetimeScope`)とテスト(`ApiServerTestBase`)はこれを使う。アプリは何もしなくても記録される
- **記録するのはエディタと開発ビルドだけ**。アクセストークンやデバイス登録の `secret_key` がそのまま見えるため(手元の開発用なので、隠さない。Console のログと同じ扱い)
- **`#if` は使わず、アセンブリで分ける**。記録の部品は `VolTech.Client.Tracing` のアセンブリに置き、asmdef の Define Constraints を `UNITY_EDITOR || DEVELOPMENT_BUILD` にする。リリースビルドでは、アセンブリごとビルドに入らない。ウィンドウと実機からの受け取りは、Editor だけの `VolTech.Client.Editor` のアセンブリに置く
- **依存の向き**: `Api`(`ApiChannel`)は `VolTech.Client.Tracing` を参照しない(リリースビルドで消えるアセンブリを参照すると、コンパイルできなくなる)。`VolTech.Client.Tracing` の側が、起動時に `ApiChannel.AddInterceptor` で記録の Interceptor を登録する(`ApiTracing.Install`)
  - Play と実機: `[RuntimeInitializeOnLoadMethod(BeforeSceneLoad)]` で、シーンを読み込む前(`LifetimeScope` が CallInvoker を作る前)に登録する
  - Play していないエディタ(EditMode テスト): `VolTech.Client.Editor` が `[InitializeOnLoadMethod]` で `ApiTracing.Install` を呼ぶ
  - 何回呼ばれても1回だけ登録する
- 実機に送る処理(`ApiTraceSender`)は、`#if !UNITY_EDITOR` で分けずに、実行時に `Application.isEditor` で決める。エディタでも毎回コンパイルされるので、壊れたらすぐに気づける
- EditMode テストの通信も記録される。テストが失敗したときに、何をやり取りしたかを見返せる

## 実機の通信を見る

USB でつないだ実機(開発ビルド)の通信を、エディタの同じウィンドウで見る。端末の画面に Viewer は作らない(小さい画面で読める UI を作るのが難しく、コピーもしにくいため)。

**Unity の PlayerConnection で送る**。プロファイラーやコンソールが、実機とエディタをつなぐときに使っている仕組み(`UnityEngine.Networking.PlayerConnection.PlayerConnection` と `UnityEditor.Networking.PlayerConnection.EditorConnection`)。開発ビルドで動き、Android は USB(ADB)、同じ PC の Windows のプレイヤーはそのまま、つなげる。自前でソケットやサーバーを用意しなくてよい。

```
端末(開発ビルド)                                 エディタ
ApiTraceInterceptor → ApiTraceStore              ApiTraceReceiver(EditorConnection)
                          │                          │
                     ApiTraceSender ──USB/ADB──→ ApiTraceStore(接続先の端末の分)
                     (PlayerConnection)              │
                                                  ApiTraceWindow(「端末」を選ぶと表示)
```

- **端末**: エディタと同じ `ApiTraceInterceptor`・`ApiTraceStore` で記録する(メモリだけ。ファイルには残さない)。`ApiTraceSender` が、記録を足したり埋めたりするたびに、その1件を PlayerConnection で送る。エディタがつながったとき(`RegisterConnection`)は、先に `Reset`(中身の無いメッセージ)を送り、続けて持っている記録をすべて送る(つなぐ前の通信も見られる)。つながっていなければ送らない
- **エディタ**: `ApiTraceReceiver` が `EditorConnection` で受け取り、端末の分の `ApiTraceStore` に入れる。同じ連番の記録が届いたら置き換える(「送信中」が「成功」に変わる)。`Reset` を受け取ったら、端末の分の記録を消す。連番は端末のアプリを起動するたびに 1 から振り直すので、消さないと前の起動の記録が上書きされて混ざる
  - 消すのは、エディタの接続のイベントではなく `Reset` で行う。接続のイベントと、端末から届く記録の順番は決まっていないので、イベントで消すと先に届いた記録まで消すことがある。同じ端末からのメッセージは送った順に届くので、`Reset` は続けて送る記録より先に処理される
- **送る形**: 1件ずつ、`JsonUtility` で JSON にした UTF-8 のバイト列。メッセージの ID は決まった `Guid` にする。Body が大きすぎるときは切り詰め、切り詰めたことを記録に残す(上限は実装で決める)
- **ウィンドウ**: Toolbar で、見るものを「Editor」と「接続先の端末」から選ぶ。接続先の端末は、Console・Profiler と同じ Unity の接続先の選択(`PlayerConnectionGUILayout.ConnectionTargetSelectionDropdown`)で選ぶ。これは IMGUI なので、`IMGUIContainer` に入れて Toolbar に置く(UI Toolkit の部品が無いため)
- 時刻は端末の時計のまま出す
- iOS の USB 接続は、macOS のエディタで使えるかを実装のときに確かめる。使えなければ、同じネットワークの Wi-Fi でつなぐ(PlayerConnection は Wi-Fi でもつながる)

## ウィンドウ(`ApiTraceWindow`)

メニュー `Window > VolTech gRPC > Request Viewer` で開く。UI Toolkit で、C# のコードだけで組み立てる(USS・UXML は使わない。色や余白は `style` で直接書く)。

```
┌ Toolbar ─────────────────────────────────────────────────────┐
│ [Clear] [● Record] [Clear on Play] [Follow latest]  [検索____] │
├──────────────────────┬────────────────────────────────────────┤
│ 一覧(ListView)       │ 詳細(ScrollView)                       │
│ 12:03:01 SignIn  OK  │ /app.v1.GachaService/Draw              │
│ 12:03:05 Draw    OK  │ 12:03:05.120  48 ms  OK                │
│ 12:03:09 Draw  INSUF…│ ▼ Request                              │
│                      │   ▼ Headers                            │
│                      │     authorization   Bearer 0a1b…       │
│                      │     x-request-id    3f2c…              │
│                      │   ▼ Body                     [Copy]    │
│                      │     { "free": false }                  │
│                      │ ▼ Response                             │
│                      │   ▼ Headers / ▼ Body / ▼ Trailers      │
│                      │ ▼ Error(エラーのときだけ)              │
│                      │     sandbox / INSUFFICIENT_GEMS        │
└──────────────────────┴────────────────────────────────────────┘
```

- **全体**: 上に Toolbar、下を `TwoPaneSplitView`(左右)で分ける
- **Toolbar**: 記録のクリア、記録の一時停止、Play を始めたら自動でクリアするか、最新の行を自動で選ぶか、RPC 名での絞り込み
- **一覧**(`ListView`): 1行に時刻・RPC の短い名前(サービス名を省く)・状態(`OK` / ステータスかエラーコードの名前 / 送信中)・所要時間。エラーの行は文字色を変える
- **詳細**(`ScrollView`): 上に RPC のフルパス・開始時刻・所要時間・状態を出し、その下に `Foldout` で Request・Response・Error を並べる
  - **Headers**: 項目ごとに1行。名前の `Label`(幅をそろえる)と値の `Label` を横に並べる。値は選択できるようにして(`selection.isSelectable`)、一部だけコピーできる
  - **Body**: 読み取り専用・複数行の `TextField` に JSON を出す(範囲を選んでコピーできる)。見出しの横の「Copy」ボタンで、JSON 全体をクリップボードに入れる(`EditorGUIUtility.systemCopyBuffer`)
  - **Trailers**(Response の下): Headers と同じ形
  - **Error**: エラーのときだけ。ステータスコード・detail・`ErrorInfo` の reason・domain と、metadata の項目ごとの行
- 一覧が増えても重くならないように、`ListView` の仮想化(`makeItem` / `bindItem`)を使う。詳細は選んだ1件だけを組み立てる

## 置き場所

| もの | 置き場所 | ビルド |
|---|---|---|
| `ApiTracing`・`ApiTrace`・`ApiTraceStore`・`ApiTraceInterceptor`・`ApiTraceMessages`・`ApiTraceSender` | `Runtime/Tracing/`(asmdef `VolTech.Client.Tracing`。Define Constraints `UNITY_EDITOR \|\| DEVELOPMENT_BUILD`) | 開発ビルドだけに入る |
| `ApiChannel.CreateCallInvoker`・`AddInterceptor` | `Runtime/Unity/ApiChannel.cs`(asmdef `VolTech.Client.Unity`) | 含む(リリースビルドでは、Interceptor を足す側が入らないので何も付かない) |
| `ApiTraceWindow`・`ApiTraceReceiver` | `Editor/`(asmdef `VolTech.Client.Editor`。Editor だけ) | 含まない |

パッケージとして分けるときは、どれも Unity 用の層([構成](structure.md))に入る。

## 実装で決めたこと

- **エラーのときは、レスポンスの Headers と Trailers が同じ中身になる**。サーバー(tonic)はエラーを Trailers-Only(ヘッダーとトレーラーを1つにまとめた形)で返し、grpc-dotnet はそれをヘッダーとしてもトレーラーとしても返すため。gRPC の仕様どおりなので、そのまま出す
- エディタで実機の記録を受け取る `ApiTraceReceiver` は、`ScriptableSingleton` にする。`EditorConnection` は呼び出し先が UnityEngine.Object のメソッドでないと登録できず、静的なメソッドやラムダを渡すと「does not derive from UnityEngine.Object」で失敗するため。ウィンドウを開いていなくても受け取るように、スクリプトを読み込むたびに `[InitializeOnLoadMethod]` で作る
- 実機の `ApiTraceSender` は、記録の追加を受け取ったら、送るのをメインスレッドに戻してから行う(`SynchronizationContext`)。記録は Interceptor の続き(別のスレッドのことがある)で足されるため。PlayerConnection はメインスレッドでしか使えないので、始めるときに `SynchronizationContext` が無ければ、警告を出して送らない
- 記録はリクエストの時点の内容で持ち、`ApiTraceStore` に入れた後は書き換えない。結果を受け取ったら、複製(`Clone`)に結果を入れて、同じ Id で置き換える(ウィンドウが別のスレッドから読んでも壊れない)
- 実機から送るときの Body の上限は 256K 文字(`ApiTraceMessages.MaxBodyLength`)。超えたら切り詰め、詳細に「Body truncated」と出す。切る位置がサロゲートペアの途中なら、その手前で切る(UTF-8 にしたときに化けないように)
- 詳細の Body の `TextField` に出すのは先頭 64K 文字まで。大きな Body をそのまま入れるとエディタが固まるため。Copy ボタンは全体をコピーする
- ウィンドウの設定(Follow latest・見るもの)は、ウィンドウのフィールドに持ち、ドメインのリロードをまたいで残す。Clear on Play は、ウィンドウを閉じていても効くように、エディタの設定(`EditorPrefs`)に持つ
- 開発ビルドとリリースビルドで、スクリプトがコンパイルできること、`VolTech.Client.Tracing.dll` が開発ビルドにだけ入ること、`Api.dll` に記録の部品が入らないことは、`PlayerBuildInterface.CompilePlayerScripts` で確かめた

## 実機で確かめる手順

USB でつないだ実機からの受け取りは、自動のテストで確かめられない。実機で使い始めるときに、次の手順で確かめる。

1. Build Settings で Development Build にチェックを入れ(Autoconnect Profiler も入れると、起動したときに自動でつながる)、Android の実機に入れる。実機からサーバーに届くように、`SampleLifetimeScope` の接続先を PC のアドレスにする(`localhost` は実機自身を指す)
2. エディタで `Window > VolTech gRPC > Request Viewer` を開き、見るものを「Player」にして、接続先の選択から実機(`AndroidPlayer(ADB@...)`)を選ぶ
3. 実機で「Start」を押し、ガチャを引く。Viewer に通信が並べば届いている

## テスト

- `ApiTraceStore`: 件数の上限で古いものから捨てること、更新で版が増えること、消した記録が結果の置き換えで戻ってこないこと(EditMode テスト)
- `ApiTraceInterceptor`: `FakeCallInvoker` を包んで呼び、成功・業務エラー・通信エラーで、ヘッダー・Body の JSON・ステータス・`ErrorInfo` が記録されること。結果と例外がそのまま呼び出し側に返ること。記録の処理で例外が起きても通信が失敗しないこと(EditMode テスト)
- 端末から送る形: `ApiTrace` を JSON にして読み戻すと、同じ中身になること。切り詰めがサロゲートペアを割らないこと(EditMode テスト)
- ウィンドウと実機からの受け取りは自動のテストをせず、サンプルを Play したときと、開発ビルドを USB でつないだときに、目で確かめる
