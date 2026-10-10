# UniGrpcKit / GameGrpcServer 設計書(ドラフト)

> 本書は、Unity と Rust のサーバーを gRPC でつなぐ基盤の設計ドラフト。基盤は、パッケージとテンプレートの2つの形で提供する想定。リポジトリの分け方と名前(UniGrpcKit・GameGrpcServer など)は後で検討し、当面は1つのリポジトリで実装する(2.6)。
>
> - **UniGrpcKit**: パッケージ。Unity のクライアントの通信基盤、サーバーの認証と状態の判定、ラッパー生成の protoc プラグイン
> - **GameGrpcServer**: サーバーのテンプレート。サーバーの基盤とアプリの雛形
>
> テンプレートは簡略化したガチャをサンプルにして作り、完成後にAtlasのAPIサーバーをgRPCへ乗せ換える(1.4)。

- ステータス: ドラフト。決まった節には【確定】を付けている。決まっていないことは9章、採らなかった案は8章、参考資料は10章にまとめている
- 実装の進め方: パッケージ・リポジトリの分け方は後で検討する。当面は1つのリポジトリで、2.6 の方針で実装を進める
- 文中の「学マス資料」「シャドバWB資料」は10章の資料を指す

| 章 | 内容 |
|---|---|
| 1 | 概要(目的、範囲、成果物、進め方、全体構成) |
| 2 | リポジトリ構成(運用、サーバーの分け方、フォルダ構成、基盤とアプリの境界、当面の実装方針) |
| 3 | API定義(`.proto` の規約。サーバーとクライアントの約束ごと) |
| 4 | サーバー基盤(Rust) |
| 5 | クライアント基盤(C# / Unity) |
| 6 | 開発運用 |
| 7 | 付録: Atlasの乗せ換え |
| 8〜10 | 採らなかった案、未確定・保留、参考資料 |

---

## 1. 概要 【確定】

### 1.1 目的

Unityクライアントと Rust APIサーバー間の通信を、REST(OpenAPI)から gRPC に置き換えるための基盤を作る。
`.proto` をAPIの唯一の定義元とし、サーバー・クライアント双方の型と通信コードを生成して、型の食い違いをなくす。
あわせて、API呼び出しに共通する処理(エラー処理、冪等性、ユーザーデータ差分の反映など)を基盤として提供する。

基盤の主な特徴:

- **再送しても一度だけ処理される**。通信が失敗しても、ユーザーが「再試行」を選べば同じRequestIDで送り直し、サーバーは保存済みの結果を返す(3.6、4.3)
- **1リクエスト = 1トランザクション**。業務処理、共通レスポンス項目の収集、レスポンスの保存をまとめてコミットする(4.3)
- **クライアントとサーバーを1セットで提供する**。gRPCを隠すラッパー、エラー処理、共通レスポンス項目(playerDiff・バッジ・通知)の反映まで基盤が持つ(5章)

### 1.2 範囲

- 対象
  - **APIサーバー(Rust)の通信方式を REST から gRPC に置き換える**。サーバーの言語は Rust のまま
  - UnityクライアントのAPI通信層
- 対象外
  - マスターデータの生成パイプライン(master-data-pipeline)。**変更しない**
  - リアルタイム対戦サーバー(C# / MagicOnion)
  - **WebGL**。gRPC-Web は使わない

### 1.3 成果物

> リポジトリと名前は、リポジトリを分けた場合の形(2.1〜2.4)。当面は1つのリポジトリで、同じ中身を実装する(2.6)。

- **パッケージのリポジトリ**(`unigrpckit`。2.2)。バージョン(タグ)を付けて配る(2.1)
  - クライアントの基盤(UPM)、サーバーの認証と状態の判定(Rust のクレート)、ラッパー生成の protoc プラグイン
- **テンプレートのリポジトリ**(`gamegrpcserver-rust`。2.2)。アプリはこれを clone して作る(2.1)
  - サーバーの基盤と、アプリの雛形(`Client/`・`Server/`・`Shared/`)を含む、そのまま開発を始められる雛形
- どちらもAtlasとは独立して管理する
- **サンプルのリポジトリ**。テンプレートを使う最初のアプリとして、普通のアプリと同じ手順(2.1)で作る
  - 中身は **簡略化したガチャ**で、**サーバーとクライアント(Unityプロジェクト)の両方**を用意する
  - API: 基盤のデバイス登録・認証に加えて、サインイン(所持データの全件取得)と、ガチャを1回引く(ジェムを消費して何か1つを付与する)
  - ガチャのレスポンスの `player_diff` にジェムの残数と入手したものを入れ、クライアントで反映する
  - クライアントに入れるもの(案。9章):
    - タイトル画面: デバイス登録とトークンの発行(`ApiCore` が自動で行う)の後、サインインで全件を取得する
    - ガチャ画面: playerDiff でジェムの残数と所持品が更新されるところと、ジェム不足(業務エラー)のダイアログを見せる
    - 基盤のインターフェースの実装例: `IErrorDialogView`・`IErrorNavigator`、`IPlayerDiffApplier`・`IBadgeApplier`・`INotificationApplier`、カスタムエラーハンドラー
    - テスト: 偽のサーバー(`FakeGachaServer` など)と、基盤の汎用のテスト(5.8)
- REST時代の成果物の残し方
  - `api-codegen`(OpenAPI → Unity向けDTO・通信クライアントの生成ツール)は、独立したリポジトリとして残す
  - AtlasのREST版(サーバーとクライアント)は、乗せ換えを始める前の状態をブランチとして残す

### 1.4 進め方

1. **パッケージとテンプレートを作る**: 続けて、テンプレートからサンプルのリポジトリを作り、ガチャを書く
2. **AtlasのREST版を残す**: 乗せ換えを始める前のコミットから、REST版を残すブランチを作る
3. **Atlasを乗せ換える**: テンプレートの基盤をAtlasに取り込み、APIを順にgRPCへ移す(7章)

- サンプルをガチャにするのは、基盤の共通処理をすべて1つの機能で試せるため
  - ジェム不足が、そのまま業務エラーの例になる(3.5)
  - 二重課金を防ぐという、冪等性が必要になる典型の例になる(3.6)
  - ジェムの減少と入手したもので差分が出る。サインインでは全件の置き換えを試せる(3.9)
  - 消費と付与を1つのトランザクションにまとめる必要がある(4.3)

### 1.5 全体構成

```
Unity Client (C#)
  ├─ 生成コード: メッセージ型 + gRPCクライアント (protoc + grpc_csharp_plugin)
  ├─ 生成コード: ラッパー (自作のprotocプラグイン)          ← パッケージのプラグインで生成(3.4、5.2)
  ├─ 共通処理: ApiCore                                      ← パッケージ(5.3)
  ├─ 認証(デバイス登録・トークン・アカウント情報の保存)    ← パッケージ(5.5)
  └─ GrpcChannel + YetAnotherHttpHandler (HTTP/2)
        │
        │ gRPC (Unary)
        ▼
Rust API Server
  ├─ 状態の判定・認証の Layer                               ← パッケージ(4.5、4.6)
  ├─ 共通処理: with_transaction()                           ← テンプレートの基盤(4.3)
  ├─ 生成コード: サービスtrait + メッセージ型 (tonic-build / prost)
  └─ MySQL
```

---

## 2. リポジトリ構成 【分け方は保留。当面は 2.6 で進める】

> 2.1〜2.4(と 1.3 のリポジトリ、6.4 の手順)は、パッケージとテンプレートを別のリポジトリにする場合の形。パッケージ・リポジトリにどう分けるかは、実装を進めて形が固まってから検討する(9章 #17)。それまでは 2.6 の方針で、1つのリポジトリで実装する。

### 2.1 運用

基盤は、**パッケージ**(バージョンを付けて参照する)と、**テンプレート**(clone してアプリが持つ)の2つの形で提供する。

| 形 | 中身 | 理由 |
|---|---|---|
| パッケージ | クライアントの基盤すべて(`ApiCore`・エラー処理・反映処理・認証・`FakeCallInvoker`・Roslyn アナライザー)、ラッパー生成の protoc プラグイン、サーバーの認証(デバイス登録・トークン・`AuthLayer`)と状態の判定(強制アップデート・メンテナンス・日付変更)、それらが使う `.proto`(`device.proto`・カスタムオプションの定義) | アプリの型に依存しない。クライアントの基盤は、生成したラッパーと差し替え口でアプリにつなぐので、UPM で配る普通の形にできる。サーバーの認証と状態の判定は、アプリごとに絶対に変えない |
| テンプレート | サーバーの基盤(`with_transaction()`・冪等性・コレクター・`AppError`)、`error_code.proto`、アプリの `.proto`、コード生成の設定、アプリの雛形 | サーバーの基盤はアプリの型(レスポンスや差分)に依存し、アプリ側で直す可能性もある |

#### パッケージ

- Rust は Git の依存(`tag` でバージョンを固定)、Unity は UPM の Git URL(タグを指定)で参照する。protoc プラグインは dotnet tool として入れる
- アプリはパッケージのコードを直接触らない。改善は、タグのバージョンを上げて取り込む
- クライアントの基盤をアプリが変えたいときは、用意した差し替え口を使う(共通エラーハンドラーの差し替え、`IErrorDialogView`・`IErrorNavigator`、反映処理、カスタムエラーハンドラー、`IAccountStore`、接続設定。5章)。足りない場面が出たら、パッケージに差し替え口を足す
- クライアントとサーバーの認証、プラグインと `ApiCore` は組で動くので、1つのリポジトリから同じタグで出す(`device.proto` や、ラッパーと `ApiCore` の食い違いが起きない)
- パッケージを直したら、タグを打ってリリースし、テンプレートとサンプルでバージョンを上げて確かめる
- パッケージはテンプレートのコードに依存しない。サーバーでアプリとつなぐのは `PlayerResolver` の trait だけ

#### テンプレート

- アプリを作るときは、テンプレートを `git clone` し、`git remote add upstream <テンプレート>` で複製元を登録する
- サーバーの基盤の改善は `git fetch upstream` で取り込む。アプリ側の変更との切り分けはブランチ運用で行う
- サンプルのリポジトリもこの手順で作り、基盤の改善を取り込む。基盤を取り込んでアプリを作る手順を、Atlasより先にサンプルで試すため
  - サンプルの業務エラーコードは、サンプルのリポジトリの `error_code.proto` にだけ入る(アプリのコードと混ざらない)
- テストは、サンプルに頼らずそれぞれのリポジトリで行う。パッケージはパッケージのリポジトリで、サーバーの基盤はテンプレートの雛形を使った結合テストで確かめる(デバイス登録・トークン・冪等性・エラー・メンテナンスの判定など。6章)

### 2.2 リポジトリ名

| もの | 名前 |
|---|---|
| パッケージのリポジトリ | **`unigrpckit`**(UniGrpcKit) |
| UPM パッケージ / C# の namespace | `com.<id>.unigrpckit` / `UniGrpcKit.Core`・`UniGrpcKit.Auth`・`UniGrpcKit.Unity` |
| サーバーのパッケージ(Rust のクレート) | `unigrpckit-auth`、`unigrpckit-status` |
| protoc プラグイン | `protoc-gen-unigrpckit` |
| `.proto` のパッケージ名(`device.proto` など) | `unigr.v1`(ユニグル。**Uni** + **gR**PC) |
| サーバーのテンプレートのリポジトリ | **`gamegrpcserver-rust`**(GameGrpcServer) |
| サーバーの基盤のクレート(テンプレートの中) | `gamegrpcserver` |
| サンプルのリポジトリ | `gamegrpcserver-rust-sample` |

- パッケージは、Unity の通信基盤が中心なので「Uni」を付ける。テンプレートは、ゲームのサーバーの雛形なので「GameGrpcServer」にする
- テンプレートは `gamegrpcserver-<サーバーの言語>` にする。サーバーの言語を増やすときは、言語ごとにリポジトリを作る(gRPC も `grpc-go`・`grpc-rust` のように言語ごとに分けている)
- パッケージは、テンプレートとは別のリポジトリにする
  - テンプレートの中に置かないのは、clone したアプリにパッケージのソースのコピーが付いてきて、参照しているものと手元のコピーがずれ得るため。更新のやり方(タグを上げる / upstream から merge する)も混ざる
  - 開発中に両方を同時に直すときは、手元のパスで参照を差し替える(6.4)
- **言語の名前を付けるのはリポジトリ名だけ**にする。Rust のクレート名、C# の namespace、`.proto` のパッケージ名には付けない
  - `.proto` のパッケージ名は、通信のパスやログに毎回出るので短くする。コードの名前は `option csharp_namespace = "UniGrpcKit.Auth";` のように指定して、長いままにする
  - `.proto` のパッケージ名は gRPC のメソッドのパス(`/unigr.v1.DeviceService/Register` など)に入る。言語の違うサーバーでも同じクライアントでつながるように全言語で同じにする。リリース後に変えると古いアプリと通信できなくなるので、実装を始める前に決める
- クライアントの基盤はパッケージなので、サーバーの言語を増やしても重複しない。言語ごとのテンプレートは、同じパッケージを参照する

### 2.3 サーバーの分け方

- **APIサーバーは1つのプロセス(モジュラーモノリス)にし、中をドメインごとのクレートに分ける**
  - 1リクエストを1つのDBトランザクションで処理する(4.3)ことが、「再送しても安全」の前提になっている。サービスとDBを分けると、この前提が分散トランザクションの問題に置き換わる
  - 開発者が1人の今は、別々にデプロイ・スケールできる利点がほぼ無い
  - 全APIにまたがる処理(認証、冪等性、エラー、メンテナンスの判定)は、サービスを分けてもどのサービスにも必要なので、基盤の規模は減らない
- 別のプロセスに分けるのは、性質(要件)が違うものだけにする。目安は「**同じトランザクションで更新しないといけないものは、同じプロセスに置く**」
  - すでに分けているもの: リアルタイム対戦(C# / MagicOnion)と、APIサーバー(Rust)
  - 今後の候補: 管理画面(使う人・認証・デプロイの時期が違う)、長くつなぎっぱなしにする機能(チャットの受信、マッチング待ちなど)
- クレートの分け方の決まり
  - ドメイン(プレイヤー、ガチャなど)ごとに1つのクレートにする。他のクレートからは `pub` にしたものしか見えないので、境界をコンパイラが守る
  - 依存は一方向にする(Cargo は循環する依存を許さない)。ドメインのクレートどうしで呼び合うのはよく、トランザクションはそのまま渡せる
  - バイナリは1つにする。将来どれかを別のプロセスに切り出すときも、境界がすでにできている
  - クレート単位で並列・差分ビルドされるので、規模が大きくなってもビルド時間を抑えやすい

### 2.4 フォルダ構成

テンプレート(アプリを作るときのいつもの形に合わせ、一番上を `Client/`・`Server/`・`Shared/` に分ける):

```
gamegrpcserver-rust/
├── Client/                         Unity プロジェクト(アプリのもの。テンプレートでは最小の雛形)
│   ├── Packages/manifest.json      パッケージを Git URL(タグ)で参照する
│   └── Assets/Generated/           .proto から生成した C#(既定の出力先)
├── Server/                         Cargo ワークスペース
│   ├── foundation/                 gamegrpcserver: AppError、with_transaction()、冪等性、差分・バッジ・通知のコレクター、
│   │                               ErrorCode(error_code.proto から生成)。サーバーの基盤(普段は触らない)
│   └── crates/
│       ├── proto/                  アプリの .proto から tonic-build で生成したコード
│       ├── <ドメイン>/              ドメインのクレート(サンプルでは player と gacha)
│       ├── api/                    gRPCのハンドラー(4.1)
│       └── server/                 バイナリ: main.rs(Layer の順番、ルーター、設定の読み込み)
├── Shared/
│   ├── proto/                      error_code.proto(予約コード入り)、アプリの .proto(既定の場所)
│   └── docs/                       本書の移管先
├── buf.yaml / buf.gen.yaml
└── docker-compose.yml / Makefile
```

パッケージ:

```
unigrpckit/
├── proto/                          device.proto、account_data.proto、カスタムオプションの定義(default_action・skip_auth)
├── unity/                          UPM パッケージ: クライアントの基盤
│   ├── Core/                       Unity に依存しない本体(ApiCore、エラー処理、反映処理、FakeCallInvoker、汎用のテスト。5.1)
│   ├── Auth/                       認証(デバイス登録・トークンの発行・アカウント情報の保存。5.5)。Unity に依存しない
│   ├── Unity/                      Unity 用の薄い層(YetAnotherHttpHandler、時計、保存先、VContainer の登録)
│   └── Analyzers/                  カスタムエラーハンドラーの Roslyn アナライザー(5.4)
├── rust/
│   ├── auth/                       unigrpckit-auth: device.proto のサービスの実装、AuthLayer、PlayerResolver の trait、
│   │                               テーブルのマイグレーション(sqlx の migrate!() で埋め込む)
│   └── status/                     unigrpckit-status: 強制アップデート・メンテナンス・日付変更の Layer、
│                                   server_status.toml の読み込み、サーバー時刻と日付のレスポンスヘッダー
└── tools/                          ラッパー生成の protoc プラグイン(dotnet tool。3.4)
```

クレートの依存の向き(サンプルの場合。左が右に依存する):

| クレート | 依存先 |
|---|---|
| `server`(バイナリ) | `api`、`unigrpckit-auth`、`unigrpckit-status`、`gamegrpcserver` |
| `api` | `gacha`、`player`、`proto`(生成コード)、`gamegrpcserver` |
| `gacha` | `player`、`gamegrpcserver` |
| `player` | `gamegrpcserver` |
| `unigrpckit-auth`・`unigrpckit-status`(パッケージ) | テンプレートのクレートには依存しない |

- **`.proto` の置き場所と、生成コードの出力先は、アプリごとに自由に変えられる**ようにする。`Shared/proto/`・`Client/Assets/Generated/` は既定の場所。パスを書くのは `buf.yaml`・`buf.gen.yaml` と、Rust の生成(`build.rs`)の設定だけにし、コードには埋め込まない
- 基盤の改善を upstream から取り込むときに触るのは `Server/foundation/`(`gamegrpcserver`)だけなので、アプリの変更と衝突しにくい。`Client/` には基盤のコードが無く、更新は `manifest.json` のタグを上げるだけ
- 新しい機能は、`Server/crates/` にドメインのクレートを足し、`Server/crates/api` にハンドラーを足す
- `Client/`・`Server/` の雛形は、サーバーが起動し、デバイス登録とトークンの発行が通るところまでを用意する(サーバー側は実装済み。2.6)
- パッケージはテンプレートの `gamegrpcserver` に依存しないので、エラーは `tonic::Status` を直接作って返す(予約コードは名前の文字列の定数で持つ。3.5)

### 2.5 基盤とアプリの境界

| もの | 置き場所 | 理由 |
|---|---|---|
| `players` テーブル | アプリ | 持たせる列はアプリごとに違う。パッケージ(当面は `Server/framework/auth`)は `device_id` から `player_id` を引くtrait(例: `PlayerResolver`)だけを定義し、アプリが実装する |
| 直前のレスポンスを保存するテーブル(4.3) | テンプレートの基盤 | `player_id` をキーにするだけで、`players` の中身には依存しない |
| `devices`・`access_tokens` テーブル(4.5) | パッケージ(マイグレーションを埋め込む) | 認証はパッケージの機能 |
| `error_code.proto` | テンプレートに予約コード入りで置き、アプリが書き足す | enum は1ファイルにまとめる必要がある。1〜999は基盤の予約コード、1000以降をアプリが足す(3.5) |
| `PlayerDiff`・`BadgeType`・`Badge`・`Notification` の中身 | アプリ | 種類はアプリごとに違う。基盤の `.proto` はアプリの `.proto` を読み込めないので、基盤はフィールド名の規約(`player_diff`・`badges`・`notifications`)と、集めて返す・反映する仕組みだけを持つ(3.9) |
| クライアントの基盤(`ApiCore` など) | パッケージ | アプリは差し替え口だけを使う(2.1) |
| エラーダイアログ | パッケージはロジック(action → ボタン → 挙動)とインターフェース、アプリは見た目(prefab)と画面遷移 | 見た目はアプリごとに違う(5.4) |
| カスタムエラーハンドラー・共通レスポンス項目の反映処理の実装 | アプリ | 何をするかはアプリごとに違う。基盤は呼び出す仕組みだけを持つ(5.4、5.6) |
| アカウント情報の保存 | パッケージに既定の実装を置き、アプリが差し替えられる | 保存方法はアプリの要件次第(5.5) |

### 2.6 当面の実装方針

パッケージ・リポジトリの分け方(2.1〜2.4)は後で検討することにし、まずは1つのリポジトリ(`grpc-foundation-samples`)で、パッケージ・テンプレート・サンプルの中身を実装する。

- 後で分けるときに切り出しやすいように、**アプリごとに変更しないもの(`framework/`)と、変更するもの(`app/`)をフォルダで分ける**
- `framework/` のクレートと `.proto` のパッケージは、**実装する機能ごとに分け、機能の名前を付ける**(`auth`・`error` など)。パッケージやリポジトリの名前(2.2)は、分けるときに決める
- `framework/` は `app/` に依存しない。それだけでビルド・テストでき、そのまま別のアプリやリポジトリに持っていける状態を保つ
- アプリとのつなぎ目は trait(`PlayerResolver` など)にし、`app/` 側で実装する。アプリの都合で `framework/` を直したくなったら、直接変えずに差し替え口を足す

```
grpc-foundation-samples/
├── Server/                     Cargo ワークスペース(1つ)
│   ├── framework/              アプリごとに変更しない
│   │   ├── auth/               デバイス登録・トークンの発行・認証。.proto(auth.v1)とマイグレーションを同梱する
│   │   └── error/              エラーのレスポンスの作り方・予約エラーコード。.proto(error.v1)を同梱する
│   └── app/                    アプリごとに変更する
│       ├── proto/              Shared/proto から生成したコード
│       ├── player/ gacha/      ドメインのクレート
│       ├── api/                gRPC のハンドラー
│       ├── server/             バイナリ
│       ├── migrations/         アプリのテーブルのマイグレーション
│       └── config/             server_status.toml
├── Unity/UnityProject/         Unity プロジェクト
├── Shared/proto/               アプリの .proto(error_code.proto など)
└── buf.yaml / buf.gen.yaml / docker-compose.yml / Makefile
```

2.4 の形との対応:

| 2.4 の形 | 当面の置き場所 |
|---|---|
| パッケージの Rust のクレート(`unigrpckit-auth` など) | `Server/framework/<機能>/`(`auth` など) |
| パッケージの `.proto`(`unigr.v1`) | 機能のクレートの中(`auth.v1`・`error.v1`) |
| テンプレートの基盤(`gamegrpcserver`) | `Server/framework/<機能>/`。作るときに機能ごとのクレートにする(`transaction` など) |
| テンプレートの `Server/crates/*` | `Server/app/*` |
| テンプレートの `Client/` | `Unity/UnityProject/` |
| クライアントの基盤(UPM)・protoc プラグイン | 未定。作るときに、同じ考え方(変更しないものを分ける)で決める |

実装の中で決めたこと:

- カスタムオプションは機能の `.proto` に置く。`ErrorAction` と `default_action` は `error/v1/options.proto`、`skip_auth` は `auth/v1/options.proto`。アプリの `error_code.proto` は `error/v1/options.proto` を import する(3.5 のコード例は、1つのファイルにまとめていた頃のもの)
- パッケージが返す予約エラーコードの名前は、`error` クレートにまとめる(`error::code`)
- `player_id` は ULID にする。DB には `BINARY(16)`、API(`.proto`)とクライアントには 26 文字の文字列で持つ。連番と違い、他人の ID や登録者数を推測されない。基盤の `PlayerId` の型も ULID(`ulid::Ulid`)にする
- buf からパッケージの `.proto` は、手元のパス(`buf.yaml` の `modules`)で参照する(9章 #3)
- Rust の生成は、使う側の `build.rs` で protox(Rust 製の protobuf コンパイラ)を使って行い、生成コードはコミットしない。protoc のインストールは要らない(9章 #15)
- `.proto` のパッケージ名(`auth.v1`・`error.v1`)と、DB に残る名前(マイグレーションの履歴のテーブル名 `_auth_migrations` など)は、リリース後には変えられないので、最初のリリースまでに確定させる。フォルダ名・クレート名・名前空間は、後からでも変えられる

---

## 3. API定義 【確定】

`.proto` で決める、サーバーとクライアントの約束ごと。実装はサーバー(4章)とクライアント(5章)に書く。

### 3.1 型の定義元

- **マスターデータの型は master-data-pipeline が決め、APIの型は `.proto` が決める。両者は別物として扱う**
- APIでマスターのIDやEnum相当の値をどの型で返すかは、そのAPIの設計として `.proto` 上で決める

### 3.2 通信方式

- **v1は Unary のみ**(リクエスト1回・レスポンス1回で完結する呼び出し)
- サーバーストリーミングは後から足す。`.proto` に `stream` 付きのrpcを足すだけで同じサービスに共存できるので、作り直しにはならない
- 「サーバーからの通知」が欲しい機能(チャット、マッチング待ちなど)は、v1ではUnaryのポーリングで作る

### 3.3 nullable / oneof

| 対象 | 規約 |
|---|---|
| スカラー(int, string等) | `optional` は使わない。「値なし」は**クライアントとサーバーで取り決めた番兵値**で表す |
| メッセージ型のフィールド | **nullを許容する**。null は「変更なし / 該当なし」を意味する。変更のないデータを送らないことで通信量を減らす |
| oneof | **ケースバイケース**で使う |

- proto3では、送られてこなかったスカラーは既定値(0 / "" / false)として受け取られる。既定値と違う番兵値(例: `-1`)を使う場合、サーバーは毎回明示的に設定する(設定漏れはエラーにならず `0` として届く)
- メッセージ型のフィールドは、C#では `null`、Rust(prost)では `Option<T>` になる
- protocが生成するC#コードには `<auto-generated>` が付くので、nullable参照型を有効にしても生成された型で警告は出ない
- 既定値が0以外のリクエストパラメータは、サーバー側で補う(必要になった時点で検討)

### 3.4 コード生成

```
*.proto(APIの唯一の定義元)
 ├─ tonic-build (build.rs) ─────────→ Rust: サービスtrait + メッセージ型
 ├─ protoc + grpc_csharp_plugin ────→ C#: メッセージ型 + クライアント
 └─ 自作のprotocプラグイン(C#) ─────→ C#: ラッパー、全RPCの一覧、共通レスポンス項目の一覧
```

- **C#のラッパー(5.2)は、自作のprotocプラグインで生成する**。`buf generate` で protoc組み込みのC#生成・grpc_csharp_plugin と一緒に実行する。Unity では Grpc.Tools が動かないので、**生成したC#コードはコミットする**
- プラグインは **C#** で書く(入出力の型 `CodeGeneratorRequest` は `Google.Protobuf` に入っている。クライアント側の道具と言語がそろう)。パッケージのリポジトリに置き、dotnet tool として配る。生成するラッパーは `ApiCore` の API に合わせるので、`ApiCore` と同じタグで出す
- 入力の `.proto` の場所と、生成コードの出力先は、アプリごとに変えられる(2.4)
- プラグインが生成するもの
  - ラッパー(gRPCサービスごとに1クラス)。RPCごとの `idempotency_level`(3.6)と、サービスごとの `skip_auth`(3.7)を読んで埋め込む
  - 全RPCの一覧(汎用のテスト用。5.8)
  - 共通レスポンス項目(`player_diff`・`badges`・`notifications`)を持つレスポンスの一覧(反映処理の登録漏れの確認用。5.6)。それらのレスポンスの `partial class` に、目印のinterface(`IHasPlayerDiff`・`IHasBadges`・`IHasNotifications`)を付ける
- **カスタムオプションは2つだけ**にする
  - `default_action`(エラーコードの既定の action。3.5)
  - `skip_auth`(認証が不要なサービス。3.7)
  - 冪等性は、標準のメソッドオプション `idempotency_level` で表す(3.6)

| オプション | クライアント | サーバー |
|---|---|---|
| `default_action` | 実行時に Descriptor から読む | 読まない |
| `skip_auth` | ラッパーの生成時にプラグインが読む | v1では読まない(4.5) |
| `idempotency_level` | ラッパーの生成時にプラグインが読む | v1から読む(標準のフィールドなので `prost-types` で読める。4.3) |

### 3.5 エラー

#### 返し方

| 種類 | ステータス | `ErrorInfo.reason` |
|---|---|---|
| 業務エラー(メンテナンス・強制アップデート等を含む) | `FAILED_PRECONDITION` | 業務ごとのコード(例: `ERROR_CODE_INSUFFICIENT_GEMS`) |
| サーバー内部エラー | `INTERNAL` | `ERROR_CODE_INTERNAL_ERROR`(詳細はログにだけ出す) |
| 認証エラー | `UNAUTHENTICATED` | なし(3.7) |

- gRPCではエラー時にレスポンスメッセージを返せないので、**リッチエラーモデル**で運ぶ。`google.rpc.Status` の details に Google 標準の `google.rpc.ErrorInfo` を入れ、トレーラー `grpc-status-details-bin` で返す(Rust は tonic-types、C# は `Google.Api.CommonProtos` で扱える)

```proto
// google/rpc/error_details.proto(Googleが提供する定義)
message ErrorInfo {
  string reason = 1;                 // ErrorCode の値の名前
  string domain = 2;                 // 固定値。クライアントは判定に使わない
  map<string, string> metadata = 3;  // action の上書き、メンテナンスの終了予定など
}
```

#### エラーコードと action の定義

エラーコードと action は `.proto` の enum で定義し、サーバー・クライアントの両方に生成する(ここが唯一の定義元)。

```proto
// error/v1/options.proto(基盤。Server/framework/error/proto に置く)
package error.v1;

import "google/protobuf/descriptor.proto";

enum ErrorAction {
  ERROR_ACTION_UNSPECIFIED = 0;
  ERROR_ACTION_STAY = 1;
  ERROR_ACTION_RETRY = 2;
  ERROR_ACTION_HOME = 3;
  ERROR_ACTION_TITLE = 4;
  ERROR_ACTION_UPDATE = 5;
}

extend google.protobuf.EnumValueOptions {
  ErrorAction default_action = 50001;
}
```

```proto
// app/v1/error_code.proto(アプリ。Shared/proto に予約コード入りで置き、アプリが書き足す)
package app.v1;

import "error/v1/options.proto";

enum ErrorCode {
  ERROR_CODE_UNSPECIFIED = 0;
  // 1〜999: 基盤の予約コード
  ERROR_CODE_NETWORK_ERROR = 1        [(error.v1.default_action) = ERROR_ACTION_RETRY];  // クライアント専用
  ERROR_CODE_INTERNAL_ERROR = 2       [(error.v1.default_action) = ERROR_ACTION_RETRY];
  ERROR_CODE_UNKNOWN_ERROR = 3        [(error.v1.default_action) = ERROR_ACTION_TITLE];  // クライアント専用
  ERROR_CODE_MAINTENANCE = 4          [(error.v1.default_action) = ERROR_ACTION_TITLE];
  ERROR_CODE_UPDATE_REQUIRED = 5      [(error.v1.default_action) = ERROR_ACTION_UPDATE];
  ERROR_CODE_ACCOUNT_SUSPENDED = 6    [(error.v1.default_action) = ERROR_ACTION_TITLE];
  ERROR_CODE_DATE_CHANGED = 7         [(error.v1.default_action) = ERROR_ACTION_TITLE];
  ERROR_CODE_CLIENT_ERROR = 8         [(error.v1.default_action) = ERROR_ACTION_TITLE];  // クライアント専用
  // 1000〜: 業務エラー(アプリが足す。機能ごとに番号の範囲を分ける)
  ERROR_CODE_INSUFFICIENT_GEMS = 1001 [(error.v1.default_action) = ERROR_ACTION_STAY];
}
```

- **`reason`** には enum の値の名前を入れる(サーバーは `as_str_name()`、クライアントは `ErrorCode.Descriptor.FindValueByName()` で相互に変換する)。名前は AIP-193 の規則(大文字のスネークケース、63文字以内)を満たす
- **既定の action** は、enum の各値にカスタムオプション `default_action` で付ける。付いていない値は Title として扱う
- **文言キー**は enum の名前から決まりで作る(例: `ERROR_CODE_INSUFFICIENT_GEMS` → `error.insufficient_gems`)
- **サーバーによる action の上書き**: 必要なときだけ `metadata["action"]` に `ErrorAction` の名前を入れる。指定できるのは5種類の action だけで、画面名は指定しない
- **知らないコード**(古いアプリに無い値)は `UNKNOWN_ERROR` と同じ挙動にする。サーバーが後からコードを足しても、古いアプリは壊れない
- **追加**は `.proto` に1行足して生成し直し、文言を足すだけ。**削除**した番号と名前は `reserved` にする(6章)
- **文言への値の埋め込み**は v1 では対応しない。例外はメンテナンスの終了予定(下記)。必要になったら `metadata` で値を送る(キー `action` は避ける)

#### action とダイアログ

| action | ボタン | 挙動 |
|---|---|---|
| Stay | OK | 閉じて画面に留まる |
| Retry | 再試行 / タイトルへ | 再試行: 同じRequestIDで再送 / タイトルへ: タイトルに戻る |
| Home | ホームへ | ホーム画面に戻る |
| Title | タイトルへ | タイトルに戻る(セッション情報を破棄) |
| Update | ストアへ | ストアのページを開く。アプリは先へ進めない |

#### 予約コード

| エラーコード(`ERROR_CODE_` を省略) | 返す側 | action | 用途 |
|---|---|---|---|
| `NETWORK_ERROR` | クライアント | Retry | 通信断・タイムアウト |
| `INTERNAL_ERROR` | サーバー | Retry | サーバー内部エラー。一時的な障害は再試行で直り得る |
| `UNKNOWN_ERROR` | クライアント | Title | 想定外のステータス、知らないコード |
| `MAINTENANCE` | サーバー | Title | メンテナンス中(4.6) |
| `UPDATE_REQUIRED` | サーバー | Update | 強制アップデート(4.6)。リリース済みのアプリには後から足せないので v1 から入れる |
| `ACCOUNT_SUSPENDED` | サーバー | Title | BAN・アカウント停止(4.5) |
| `DATE_CHANGED` | サーバー | Title | 日付変更(4.6)。サインインし直し、所持データを全件取り直す |
| `CLIENT_ERROR` | クライアント | Title | クライアントのバグ(共通レスポンス項目の反映処理での例外など。5.6)。通信エラーとは違う文言にする |

- パッケージ(4.5、4.6)が返す予約コード(`MAINTENANCE`・`UPDATE_REQUIRED`・`DATE_CHANGED`・`ACCOUNT_SUSPENDED`)は、基盤の `error` クレートに名前の文字列の定数としてまとめて持つ(`error::code`。`ErrorCode` の enum はアプリ側で生成されるため)。テンプレートの `error_code.proto` の予約コードと一致しているかを、テストで確かめる(6.3)

#### メンテナンス中の表示

- `MAINTENANCE` を返すときは、終了予定時刻などの表示用の値を `metadata` に入れる。クライアントはそれをダイアログに表示する
- メンテナンスの判定は全リクエスト共通(4.6)なので、タイトル画面で最初に呼ぶトークン発行でも同じ値が返る。メンテナンスを知らせる専用のAPIは作らない
- 運営からの「お知らせ」は、認証ありの普通のAPIで取得する(静的なファイルでは配信しない)。トークン発行は認証不要なので、タイトル画面でも先にトークンを取ってから呼べる
- メンテナンス中でも呼べるAPI(判定の対象外)は v1 では作らない。メンテナンス中はお知らせのAPIも `MAINTENANCE` になるが、`metadata` の値で必要な情報は表示できる

### 3.6 冪等性(RequestID と idempotency_level)

学マス資料 p.51 のレスポンスキャッシュを参考にした仕組み。レスポンスを受け取る前に通信が切れて再送しても、処理は一度だけ行われる。

- クライアントは1回の操作につき1つRequestIDを採番し、メタデータ `x-request-id` に付けて送る(UUID、ハイフン無しの32文字)。「再試行」では同じRequestIDで再送する
- サーバーは同じRequestIDが再送されたら、処理し直さずに保存済みのレスポンスを返す(仕組みは4.3)
- 対象かどうかは、RPCごとに標準のメソッドオプション `idempotency_level` で表す

| 値 | 意味 | 扱い | 例 |
|---|---|---|---|
| `IDEMPOTENCY_UNKNOWN`(書かなければこれ) | 冪等か分からない | **対象**。RequestIDでチェックし、クライアントは1つずつ送る | ガチャ、編成の保存 |
| `NO_SIDE_EFFECTS` | 副作用が無い(読み取り) | 対象外。読み取り専用のトランザクションで実行する | ポーリング、一覧の取得 |
| `IDEMPOTENT` | 何回実行しても結果が同じ | 対象外 | サインイン、デバイス登録・認証、サインアップ |

```proto
service GachaService {
  rpc Draw(DrawRequest) returns (DrawResponse);   // 何も書かない → 対象
  rpc GetBanners(GetBannersRequest) returns (GetBannersResponse) {
    option idempotency_level = NO_SIDE_EFFECTS;   // 読み取りだけ → 対象外
  }
}
```

- 書かなければ「対象」になるので、書き忘れても安全な側に倒れる(外し忘れは書き込みが少し増えるだけだが、付け忘れは二重実行になる)
- `IDEMPOTENT` の宣言が正しいかは機械的に判定できないので、`.proto` の差分をレビューで確認し、そのAPIの結合テストで確かめる(6章)
- **サインイン**は `IDEMPOTENT` にする。全件データを返すので、対象にすると保存量とサインインが集中する時間帯の書き込みが重くなる。読み取りが中心なので、再実行されても結果は変わらない
  - 付与などの状態変更(ログインボーナス等)は別のAPIにして、この仕組みに乗せる(再送されても二重に付与されず、保存済みのレスポンスで演出も出せる)。サインイン中の書き込みは、最終ログイン日時の更新のように何回実行しても同じものに限る
- **認証前のAPI**(デバイス登録など)は、どのプレイヤーか決まらないので、APIごとに冪等に作って `IDEMPOTENT` と宣言する
- **サインアップ**(プレイヤーの作成)は、プレイヤーの行がまだ無いのでこの仕組みを使えない。`IDEMPOTENT` とし、**作成済みならエラーにせず作成済みのプレイヤーを返す**

### 3.7 認証

**Atlasのデバイス認証(ゲスト型)を引き継ぐ。**

1. **デバイス登録**: クライアントが生成した `secret_key` を送り、サーバーが `device_id` を発行する
2. **トークンの発行**: `device_id` と `secret_key` を送り、アクセストークンを受け取る。有効期限は1時間で、有効なトークンは1デバイスにつき1つ(発行し直すと古いものは無効になる)
3. **要認証のAPI**: メタデータ `authorization: Bearer <token>` を付けて呼ぶ。無い・存在しない・期限切れなら `UNAUTHENTICATED` が返る
4. **サインイン**: 「プレイヤー未作成」(例: `PLAYER_NOT_FOUND`)が返ったら、サインアップ(プレイヤーの作成)へ進む。サインアップはデバイス登録とは別の機能・別のAPI

- **認証の要否は、サービスのカスタムオプション `skip_auth` で表す**。v1で `skip_auth` を付けるのは、基盤のデバイス登録・認証のサービスだけ
- BAN・アカウント停止は `FAILED_PRECONDITION` + `ACCOUNT_SUSPENDED` で返す(再認証しても解決しないので `UNAUTHENTICATED` にはしない)
- gRPCのメタデータのキーは小文字にする

### 3.8 メタデータ(ヘッダー)

| 向き | 内容 | 用途 |
|---|---|---|
| リクエスト | `authorization: Bearer <token>`(`skip_auth` のサービスには付けない) | 認証(3.7) |
| リクエスト | `x-request-id` | 冪等性(3.6)。対象のRPCだけ |
| リクエスト | プラットフォーム(iOS / Android / Windows)と、整数のビルド番号 | 強制アップデートの判定(4.6) |
| リクエスト | ゲーム内の日付(セッションの最初に受け取ったもの) | 日付変更の判定(4.6)。トークン発行とサインインには付けない |
| レスポンス | サーバー時刻 | クライアントのサーバー時刻の同期(5.3) |
| レスポンス | 今のゲーム内の日付 | 日付変更の判定(4.6) |

- `x-request-id` と `authorization` 以外のキー名は、実装時に決める(9章)

### 3.9 共通レスポンス項目

APIのレスポンスに共通して載せる項目。決まった名前のフィールドで持たせ、サーバーは集めて詰め、クライアントは自動で反映する。

| | `player_diff` | `badges` | `notifications` |
|---|---|---|---|
| 何か | DBのデータの変更。クライアントのキャッシュをサーバーに合わせる | DBのデータから計算した、表示用の状態 | リクエスト中に起きた出来事(報酬を得た、ミッションをクリアした) |
| DBに保存するか | する(業務データそのもの) | しない(その場で計算する) | しない |
| クライアントでの扱い | Repositoryを更新する | バッジの状態を更新する | 待ち行列に積み、画面が都合のよいときに出す |

```proto
// アプリの .proto(中身はアプリごとに違う。例はサンプルのもの)
message DrawResponse {
  PlayerDiff player_diff = 1;                // 規約: 名前は player_diff
  repeated Badge badges = 2;                 // 規約: 名前は badges
  repeated Notification notifications = 3;   // 規約: 名前は notifications
}

message PlayerDiff {
  ItemsDiff items = 1;         // null = 変更なし(3.3)。リソース種別ごとに1フィールド
}
message ItemsDiff {
  repeated PlayerItem upserted = 1;  // 追加・更新されたもの(変更後の値)
  repeated int32 removed = 2;        // 削除されたもののID
}

enum BadgeType {
  BADGE_TYPE_UNSPECIFIED = 0;
  BADGE_TYPE_PRESENT = 1;      // カウントバッジ
  BADGE_TYPE_INFORMATION = 2;  // 通常バッジ
  BADGE_TYPE_SHOP = 3;
  BADGE_TYPE_GACHA = 4;
}
message Badge {
  BadgeType type = 1;
  bool visible = 2;
  int32 count = 3;   // カウントバッジだけ。通常バッジは0
}

message Notification {
  oneof event {
    RewardReceived reward_received = 1;
    MissionCleared mission_cleared = 2;
  }
}
```

- 3つとも、エラーのレスポンスには付かない。レスポンスごと保存される(4.3)ので、同じRequestIDで再送されても同じ内容が返り、通知が2回出ることはない
- **player_diff**
  - 値は変化量(ジェム-100)ではなく、**変更後の値**(ジェム残り200)で持つ。同じ差分を2回反映しても結果が変わらない
  - サインインは全件のスナップショットを返し、クライアントは差分としてではなく「置き換え」として反映する
- **badges**
  - 含まれない種類は「変化なし」
  - 種類は `.proto` の enum で持つ(種類を足すときはサーバーに判定処理を書くので、マスターで持つ利点が無い)。古いアプリは知らない種類を無視する
  - クライアントが同期しているデータから判定できるバッジ(例: 未受け取りのプレゼントの件数)は、サーバーから返さない。サーバーから返すのは、クライアントがデータを持っていないもの(お知らせ、ショップなど)だけ
- **notifications**
  - 出来事の種類ごとに中身が違うので `oneof` で表す。古いアプリは知らない種類を無視する
  - 「通知」は、運営からの「お知らせ」(3.5)とは別物として呼び分ける
- 共通の項目を後から足すときも、同じ形(規約のフィールド名、サーバーのコレクター、クライアントの反映処理、使っていれば必須)で増やす

---

## 4. サーバー基盤 【確定(クレートの細かい分け方を除く)】

### 4.1 レイヤー構成

```
Server/app/gacha/src/         ドメインのクレート(サンプルの例。player も同じ形)
├── service/                   業務ロジック本体。&mut Transaction とコレクター(4.4)を引数で受け取る
├── model/                     各エンティティのstruct。モデル自身の状態変化だけで完結する処理
│                              (経験値からレベルを計算する等)はここのimplブロックに置く
└── lib.rs                     他のクレートに見せる関数・型だけを pub にする

Server/app/api/src/
└── gacha.rs                   薄いgRPCハンドラー(tonic生成traitの実装)。Proto⇔ドメインの変換、
                               with_transaction() の呼び出し、Responseの組み立てだけを行う
```

- `api` → `service/` → `model/` の3層にする。`api` はアプリに1つのクレートとして置き、`service/` と `model/` はドメインのクレートの中に置く
- モデル自身で完結する処理は、別のディレクトリに分けず、その型のimplブロックに書く(Rustには `partial class` が無く、分けると型の定義とロジックが離れて追いにくい)
- **`service/` の書き込み系の関数は `&mut Transaction<'_, MySql>` を引数に要求する**。トランザクションを持たない呼び出し元からはコンパイルが通らず、冪等性の仕組み(4.3)を通さずに書き込むことを型で防ぐ

### 4.2 リクエストの処理の流れ

| 順 | 処理 | 実装 | 返すもの |
|---|---|---|---|
| 1 | ログ | `tower_http::TraceLayer` などの汎用Layer | |
| 2 | 強制アップデートの判定 | 状態の判定のクレート(4.6)の tower Layer(DBを使わない) | `UPDATE_REQUIRED` |
| 3 | メンテナンスの判定 | 状態の判定のクレート(4.6)の tower Layer(DBを使わない) | `MAINTENANCE` |
| 4 | 認証 | `auth` クレートの `AuthLayer`(非同期の `tower::Service`。4.5)。`skip_auth` 以外のサービスにだけルーターで付ける | `UNAUTHENTICATED`・`ACCOUNT_SUSPENDED` |
| 5 | 日付変更の判定 | 状態の判定のクレート(4.6)の tower Layer(DBを使わない) | `DATE_CHANGED` |
| 6 | ハンドラー | `with_transaction()` を呼ぶ(4.3) | 成功、または業務エラー |

- 強制アップデートをメンテナンスより先に判定する。メンテナンスと同時に新しいバージョンを出すとき、古いアプリのユーザーにメンテナンス中のうちにアップデートを促せる
- 2・3・5はDBを使わないので、メンテナンスでDBを止めていても判定できる
- 認証は、トークンの検証にDBへの問い合わせが要るので、非同期の `tower::Service` で書く(`tonic::Interceptor` は同期関数専用)
- 冪等性のチェック・トランザクション・コレクターは、ミドルウェアにせず、ハンドラーが `with_transaction()` を明示的に呼ぶ。これらはレスポンスの型に依存する処理で、tower Layer で汎用的に書くと、gRPCの成否がHTTP/2のtrailerに乗ることや、レスポンスのFuture型が統一できないことなどの制約にぶつかる。また、sqlx の `Transaction` は `Sync` ではないので、`http::Extensions` に載せて渡せない
- エラーの変換(`AppError` → `tonic::Status`)は、`with_transaction()` やハンドラーの戻り値の変換で行う
- 成功したレスポンスには、サーバー時刻と今のゲーム内の日付をヘッダーで付ける(3.8)
- **タイムアウト**: tonic は、クライアントの deadline(`grpc-timeout`)とサーバー側のタイムアウトのうち短い方で処理を打ち切る。打ち切られたトランザクションは sqlx が自動でロールバックするので、再送しても安全

### 4.3 with_transaction() と冪等性

`with_transaction()` は次を行う。

1. トランザクションを開始する(`NO_SIDE_EFFECTS` のRPCは読み取り専用のトランザクション)
2. 冪等性の対象のRPCなら、プレイヤーの行を `SELECT ... FOR UPDATE` でロックし、保存済みのRequestIDを読む。同じRequestIDなら、保存済みのメソッド名と一致するかを確かめ、一致すれば保存済みのレスポンスを返して終わる(一致しなければエラー)
3. ハンドラー本体(`service/` の関数)を、同じトランザクションとコレクターを渡して呼ぶ
4. 成功したら、コレクターの中身を `player_diff`・`badges`・`notifications` に詰める(4.4)。対象のRPCなら、RequestID・メソッド名・レスポンスを保存(上書き)する。そしてコミットする
5. 失敗したらロールバックする。エラーのレスポンスは保存しないので、再試行すると処理し直す

| 決めたこと | 理由 |
|---|---|
| プレイヤーごとに**最新1件だけ**保存する(MySQLのテーブルに1プレイヤー1行、上書き) | 保存量がプレイヤー数で頭打ちになり、期限切れの掃除が要らない。クライアントは対象のRPCを1つずつ送る(5.3)ので、最新1件で足りる |
| 行を `SELECT ... FOR UPDATE` でロックする | 処理中に再送が届いても、最初の処理の完了を待ってから保存済みのレスポンスを返せる |
| レスポンスの保存と業務データの更新を、同じトランザクションで行う | 「業務データはコミット済み・レスポンスは未保存」の状態が生まれず、再送で二重実行されない |
| **メソッド名**も保存し、同じRequestIDで別のメソッドが呼ばれたらエラーにする | クライアントの不具合でRequestIDが使い回されたとき、型の違うレスポンスを返さない |
| `NO_SIDE_EFFECTS` は読み取り専用のトランザクションで実行する | 誤って書き込むとMySQLがその場でエラーを返すので、宣言の間違いに開発中に気づける |
| 対象かどうかは、サーバーも v1 から `.proto` の `idempotency_level` を読んで決める(`prost-types`) | クライアントとサーバーで判定がずれない。ハンドラーはすべて同じように `with_transaction()` を呼べばよい |

- ロックの対象にするため、プレイヤーの作成時に保存用の行も1行作っておく

実装で決めたこと(`Server/framework/transaction`):

- `with_transaction()` は `Transactions::run(&request, player_id, handler)` として基盤のクレートに置く。テーブル `last_responses` とマイグレーションもクレートに持つ(履歴は `_transaction_migrations`)
- 冪等性の表は、起動時にアプリの FileDescriptorSet の `idempotency_level` から作る(`IdempotencyLevels`)。表に無い RPC は対象として扱う
- tonic はサーバーのハンドラーに RPC の名前を渡さないので、全サービスの外側に `RpcMethodLayer` を付け、HTTP のパスから取り出した名前を extensions に入れる
- 保存用の行は、サインアップで作るのに加えて、無ければ最初の対象のリクエストで作る(この仕組みより前に作られたプレイヤーのため)
- 冪等性の対象の RPC で `x-request-id` が無い・形が違う(英数字とハイフンの1〜64文字でない)、または同じ RequestID で別のメソッドが呼ばれたら、`INVALID_ARGUMENT` を返す(クライアントでは `UNKNOWN_ERROR` になる)

### 4.4 コレクター(差分・バッジ・通知)

共通レスポンス項目(3.9)は、リクエスト単位のコレクターに集め、`with_transaction()` の最後にレスポンスへ詰める。ハンドラーは手で並べない。トランザクションがロールバックされたら中身も捨てる。

| コレクター | 誰が記録するか | 詰めるもの |
|---|---|---|
| 差分(`DiffCollector`) | 書き込み関数(アイテムを増やす等)が、DBに書くついでに変更後の値を記録する | `player_diff`。同じものが複数回書き換わったら最後の値だけ |
| バッジ(`BadgeCollector`) | 書き込み関数が、影響するバッジの種類を `mark` で記録する(例: プレゼントを受け取る関数は `Present`)。サインインとホームは `mark_all` で全種類 | `badges`。記録された種類だけ、アプリが実装した判定(trait `BadgeResolver`)で今の状態を計算する |
| 通知(`NotificationCollector`) | 業務の処理が出来事を積む(例: ミッションの条件を満たしたら「ミッションをクリアした」) | `notifications` |

- 新しいAPIが同じ書き込み関数を使えば、差分もバッジも自動で返る。「どのAPIでどのバッジを返すか」の表は持たない
- 運営の操作で変わるバッジ(新しいお知らせの公開など)はリクエストでは記録されないが、サインインとホームで全種類を返すので、ホームに戻るたびに更新される
- `service/` の関数はコレクターを引数で受け取る(まとめて1つの構造体にしてもよい)
- 基盤が持つのはコレクターと詰める処理、`BadgeResolver` の trait。バッジの判定はアプリが実装する

当面の実装(基盤とアプリの分け方を決めるまでの形):

- 差分のコレクターは、アプリの `player` クレートに置く(`player::diff::PlayerDiffCollector`)。ジェム・アイテムを書き換える関数がコレクターを受け取り、変更後の値を記録する
- レスポンスへの詰め込みは、ハンドラーが `Transactions::run` に渡す処理の最後で行う(`api::diff::to_proto`)。`with_transaction()` が自動で詰める形は、基盤に置く範囲を決めるときに見直す
- バッジ・通知のコレクターは、使う API ができてから足す

### 4.5 認証

パッケージの `auth` クレート(当面は `Server/framework/auth`)が持つ。アプリとのつなぎ目は `PlayerResolver` の trait だけ。

- テーブル(`devices`・`access_tokens`)のマイグレーションはクレートに埋め込み(sqlx の `migrate!()`)、アプリは起動時に実行する
- **デバイス登録**: `secret_key` は Argon2 でハッシュ化して保存する(平文は保存しない)
- **トークン**: ランダムな文字列(Atlasは ULID を2つ連結)。`device_id` と有効期限と一緒にDBに保存する。有効期限は1時間、1デバイスにつき1つ(発行し直すと上書き)
- **検証**: `AuthLayer` がメタデータ `authorization` を検証する。BAN・アカウント停止もここで判定する
- トークンは有効だがプレイヤーがまだいない状態(サインアップの前)を、`PlayerResolver`(2.5)が扱えるようにする
- **`skip_auth` は v1 では読まず、ルーターで要認証のサービスにだけ `AuthLayer` を付ける**。付け分けのずれは結合テストで確かめる(6章)。ずれても `UNAUTHENTICATED` になってすぐ気づける(冪等性のずれのように、エラーが出ないまま二重実行になることは無い)

### 4.6 サーバーの状態(メンテナンス・強制アップデート・日付変更)

パッケージの状態の判定のクレートが持つ(未実装。`Server/framework/` に作り、名前は作るときに機能の名前で決める)。サーバー時刻とゲーム内の日付のレスポンスヘッダー(3.8)も、ここで付ける。

#### 設定

DBの外の1つのファイルに置き、サーバーが数秒おきに読み直す(再起動せずに切り替えられる)。

```toml
# server_status.toml
[maintenance]
enabled = false
end_at = "2026-10-01T15:00:00+09:00"   # ダイアログに出す終了予定(省略可)

[min_version]   # プラットフォームごとの最低ビルド番号
ios = 120
android = 120
windows = 120

[game_day]
reset_hour = 4            # ゲーム内の日付が変わる時刻
timezone = "Asia/Tokyo"
```

- 読み込みに失敗したら、直前の状態を使い続ける。起動時に一度も読めなかったら、起動しない
- 読み込み元は trait にする。AWSに載せるときは、AWSの設定サービス(SSM Parameter Store など)から読む実装に差し替える(サーバーが複数台になっても状態がそろう)
- DBに置かないのは、メンテナンス中はDBを止めることがあり、判定できなくなる(`MAINTENANCE` ではなく `INTERNAL` が返る)ため

#### 判定

- **メンテナンス**: v1はオン/オフだけ。オンなら全員に `MAINTENANCE` を返し、`end_at` があれば `metadata` に入れる。時刻が来ても自動では終わらず、フラグをオフにして終える。通常メンテと緊急メンテの区別はしない(緊急のときは終了予定を空にする)
- **強制アップデート**: リクエストのプラットフォームとビルド番号(3.8)を、プラットフォームごとの最低ビルド番号と比べ、小さければ `UPDATE_REQUIRED` を返す。整数なので比べ間違いが起きない
- **日付変更**: リクエストのゲーム内の日付が今の日付と違えば `DATE_CHANGED` を返す。**日付のヘッダーが無ければ判定しない**ので、トークン発行とサインインは判定に引っかからない(APIごとの除外の指定は要らない)。クライアントが日付を偽っても、ログインボーナスなどの切り替えはサーバーの時刻で判定するので、不正にはつながらない

#### 後から足すもの

v1は、サーバー1台でエンジニアが自分で運用する前提の最小構成にしている。

| 時期 | 足すもの | 理由 |
|---|---|---|
| 一般のユーザーに公開する前 | ソフトメンテ(IPの許可リストの人だけ通す) | メンテ明けに、本番環境で動作確認してから公開するため。IPだけならDBを使わずに判定できる |
| 一般のユーザーに公開する前 | 設定の置き場所をAWSの設定サービスにする | サーバーが複数台になっても状態をそろえるため |
| 運用が大きくなってから | `player_id` の許可リスト | 認証の後にもう一度判定する形になる |
| 運用が大きくなってから | 通常メンテと緊急メンテの出し分け | 予約コードを分ければ、文言が自動で分かれる |
| 運用が大きくなってから | 管理画面 | エンジニア以外が操作でき、変更の記録を残すため |

- IPの許可リストでは、ALBが付けた `X-Forwarded-For` の**末尾**の値を使う。ALBは既にある値の末尾に接続元のIPを足すので、先頭の値はクライアントが自由に書ける。信頼できる中継が他にもある場合は、「末尾から何番目か」を設定で持つ
- メンテナンスの判定の対象外にするAPIが必要になったら、`skip_auth` と同じくサービスのカスタムオプションで表す

---

## 5. クライアント基盤 【確定】

基盤は、**gRPCを呼び出し側から隠すラッパー**と、**共通処理をまとめた `ApiCore`** を提供する(DeNAの内製基盤Takashoと同じ考え方)。型名(`ApiCore`、`ApiResult` など)は、実装するときに調整してよい。

### 5.1 構成

#### 依存パッケージ

| パッケージ | 用途 |
|---|---|
| Grpc.Net.Client(と、依存の Grpc.Core.Api) | gRPCクライアント。`RpcException`・`Metadata` などの型 |
| Google.Protobuf | 生成されたメッセージ型のランタイム |
| Google.Api.CommonProtos | エラー詳細の型(`google.rpc.Status` / `google.rpc.ErrorInfo`) |
| UniTask | 非同期処理。基盤のAPI(ラッパー、`ApiCore`、各interface)は UniTask を返す |
| YetAnotherHttpHandler | UnityでHTTP/2通信を行うためのハンドラー(Unity用の層だけが使う) |

- YetAnotherHttpHandler は OpenUPM、それ以外は Unity NuGet レジストリ(`org.nuget.*`)から入れる
- エラー詳細を読む `Grpc.StatusProto` は Unity NuGet レジストリに無いので、トレーラーを読んでパースする処理を自前で書く

#### Unityに依存しない本体(Core)と、Unity用の層

> 下の置き場所(`unigrpckit/unity/`)と名前(`UniGrpcKit.*`)は、リポジトリを分けた場合の形。当面の置き場所と名前は、クライアント側を作るときに 2.6 の方針(変更しないものを分け、機能の名前を付ける)で決める。

```
unigrpckit/unity/
├── Core/     本体。ApiCore、エラー処理、共通レスポンス項目の反映、
│              FakeCallInvoker、汎用のテスト、
│              HttpMessageHandler・時計などを受け取る interface
├── Auth/     認証(5.5)
├── Analyzers/ Roslyn アナライザー(5.4)
└── Unity/    薄い層。YetAnotherHttpHandler の設定、realtimeSinceStartup を使う時計、
               保存先のパス(Application.persistentDataPath)とバンドルID(Application.identifier)を Auth に渡す処理、
               VContainer の登録
```

- 本体が依存するのは UniTask、Grpc.Net.Client、Google.Protobuf、Google.Api.CommonProtos だけ
- 本体は Unity の外(csproj)ではビルドしない。Unity 以外で使う予定が無いので、asmdef だけで持つ。本体のテストも Unity の EditMode テストで回す(6.3)。Unity のライセンスが要るので、v1 では CI では回らない
- 認証(デバイス登録・トークンの発行・アカウント情報の保存)は Auth にある(5.5)。Auth も、下の本体の決まりを守る
- 生成したラッパーと C# のメッセージ型は、アプリの Unity プロジェクトの中(既定は `Unity/UnityProject/Assets/Scripts/Api/Generated/`)に出力する
- **本体の決まり**
  1. UnityEngine を使わない。本体の asmdef で「No Engine References」を有効にし、使った時点でコンパイルエラーにする
  2. `Task` を UniTask にするときは、`AsUniTask()` を使わずにそのまま `await` する。Unity 6 の UniTask は Unity の `Awaitable` 用の `AsUniTask()` も持っていて、どれを使うかを決めるのに UnityEngine の型が要り、No Engine References の asmdef ではコンパイルエラーになる。それ以外の UniTask の機能(`UniTaskCompletionSource`・`WhenAll`・`Delay` など)は使える(EditMode テストで確かめた)
  3. タイムアウトは gRPC の deadline で扱う(5.7)

### 5.2 ラッパー

```
Serviceクラス(アプリ)
  │  gachaApi.DrawAsync(request) → ApiResult<DrawResponse>
  ▼
ラッパー(自作のprotocプラグインで生成。gRPCサービスごとに1クラス)
  │  RPCを呼ぶ処理を static ラムダで渡す
  ▼
ApiCore(基盤)  共通処理(5.3)
  │
  ▼
生成されたgRPCクライアント(grpc_csharp_plugin) → GrpcChannel + YetAnotherHttpHandler
```

```csharp
// 自動生成されるラッパー(イメージ)
public sealed partial class GachaApi
{
    public UniTask<ApiResult<DrawResponse>> DrawAsync(
        DrawRequest request, IReadOnlyList<ErrorCode> handledCodes = null)
        => core.InvokeAsync(DrawSettings, client, request, handledCodes,
            static (c, req, options) => c.DrawAsync(req, options));   // options: ヘッダーとdeadline
}

// 使う側(アプリのServiceクラス)
var result = await gachaApi.DrawAsync(new DrawRequest { BannerId = bannerId });
if (!result.IsSuccess) return;   // 失敗時の共通の処理(ダイアログ等)は済んでいる
// result.Value を使う
```

- 呼び出し側(アプリ)は、生成されたgRPCクライアント、`Metadata`、`RpcException` に触れない
- **gRPCサービスごとに1クラス、RPCごとに1メソッド**。クラス名は gRPCサービス名から決める(例: `GachaService` → `GachaApi`。規則は9章)
- メソッド名は、生成クライアントと同じ **`{RPC名}Async`**。`.proto` から呼び出し箇所までを名前で追える
- ラッパーは、`ApiCore` に「RPCを呼ぶ処理」を **`static` ラムダ**で渡すだけ。キャプチャしないラムダは delegate が使い回されるので、呼び出しごとのアロケーションは起きない。`static` を付けると、うっかりキャプチャしたときにコンパイルエラーになる
- RPCごとの設定(冪等性の対象か)と、サービスごとの設定(認証が要るか)は、プラグインが `.proto` から読んで埋め込む(3.4)
- **一度呼んだ通信はキャンセルできない**。ラッパーは `CancellationToken` を受け取らず、`ApiCore` は共通レスポンス項目の反映まで必ず最後まで行う
  - 送信後に打ち切ると、サーバーでは処理が終わっているのにクライアントには反映されない(例: ガチャを引いた直後に画面を閉じると、ジェムは減っているのにクライアントのデータは変わらない)
  - 呼び出した画面が閉じられたときは、通信は止めず、呼び出し側が待つのをやめるだけにする(UniTask の `AttachExternalCancellation`)
  - 応答が返ってこない場合は、deadline(5.7)で扱う

### 5.3 ApiCore の共通処理

クライアントの共通処理は、すべて `ApiCore` に置く。v1では gRPC の Interceptor は使わない(生成したラッパーを必ず通るので、処理の入口を1つにできる)。ログや計測を gRPC の層で取りたくなったら、そのときに足す。

| # | 処理 | 補足 |
|---|---|---|
| 1 | 順番待ち | 冪等性の対象(`IDEMPOTENCY_UNKNOWN`)のRPCだけ。成功するか、ユーザーが諦めるまで、次を送らない。応答が届かないまま次を送ると、サーバーの記録が上書きされ、再送で二重実行になるため |
| 2 | RequestIDの採番 | 冪等性の対象のRPCだけ。再送のループの外側で採番し、再送では同じIDを使う |
| 3 | トークンの取得 | Auth を呼ぶ。デバイス登録・発行・期限の確認はパッケージが行う(5.5) |
| 4 | ヘッダーの付与 | 3.8 |
| 5 | エラーの変換 | 5.4 |
| 6 | `UNAUTHENTICATED` のときの再認証(Auth にトークンを発行し直させる)と、1回だけの再送 | 5.5 |
| 7 | ログ | |
| 8 | エラー時にエラーハンドラーを待ち、再送するか諦めるかを受け取る | 5.4 |
| 9 | 通信中の入力ブロックと、通信中の表示 | アプリ側のフックを呼ぶ。表示するかは呼び出しごとに指定でき、既定はオン |
| 10 | タイトル・ホームに戻るときの順番待ちの破棄 | 待っているリクエストは送らずに、失敗として終わらせる |
| 11 | サーバー時刻の同期 | レスポンスヘッダーのサーバー時刻と、Unity用の層が渡す時計(`realtimeSinceStartup`)から今のサーバー時刻を計算し、アプリに提供する(`IServerClock` など)。端末の時計は使わないので、端末の時刻を変えられても影響を受けない |
| 12 | 日付の保持 | セッションの最初に受け取ったゲーム内の日付を覚え、以降のリクエストに付ける(3.8)。タイトルに戻るとセッションと一緒に消える |
| 13 | 共通レスポンス項目の反映 | 5.6 |
| 14 | 反映処理などで例外が起きたときの後始末 | 5.6 |

- チャネル(`GrpcChannel` + YetAnotherHttpHandler)は1つを使い回す

### 5.4 エラー処理

#### 受け取ったエラーの変換

受け取ったエラーをエラーコード(3.5)に変換し、以降はコードだけで扱う。

| 受け取ったもの | エラーコード |
|---|---|
| `FAILED_PRECONDITION` + `ErrorInfo` | `reason` を `ErrorCode` に戻したもの(知らない名前なら `UNKNOWN_ERROR`) |
| `INTERNAL` | `INTERNAL_ERROR`(`ErrorInfo` が無くても) |
| `UNAVAILABLE` / `DEADLINE_EXCEEDED` | `NETWORK_ERROR` |
| `UNAUTHENTICATED` | 再認証して1回だけ送り直す。それでも失敗したらエラー(5.5) |
| クライアント内部の例外(共通レスポンス項目の反映処理など) | `CLIENT_ERROR`(5.6) |
| 上記以外(`ErrorInfo` の無い `FAILED_PRECONDITION`、`UNIMPLEMENTED` 等) | `UNKNOWN_ERROR` |

- 存在しないメソッドを呼ぶと `UNIMPLEMENTED` になるので、業務上の「見つからない」と混ざらない

- action は、`metadata["action"]` があればそれ、無いか知らない値なら `default_action`(起動時に Descriptor から読んで `Dictionary<ErrorCode, ErrorAction>` にしておく)
- 文言と action の取り出しは `IErrorPolicyProvider` のようなインターフェースの裏に置き、将来マスターから読む形にも差し替えられるようにする

#### 呼び出し側への返し方

- **Result型(`ApiResult<T>`、`readonly struct`)**で返す。呼び出し側に例外は投げない
  - 失敗の多くは「共通の処理が済んだので抜けるだけ」で、毎回 `try/catch` を書くのは大げさ。業務エラーは想定内の結果で、例外的な出来事ではない
  - 失敗時に `Value` を読んだら例外を投げる。成功の確認し忘れに、開発中に気づける
  - Grpc.Net.Client はOK以外で必ず `RpcException` を投げるので、性能の差は呼び出し側に投げ直すかどうかだけで、通信のコストと比べれば無視できる

#### 自動リトライはしない

- 再送するかどうかは、必ずユーザーがエラーダイアログで選ぶ。`ApiCore` はエラーハンドラーを待ち、「再試行」なら同じRequestIDで送り直す
- 例外は `UNAUTHENTICATED` だけ(5.5)
- Grpc.Net.Client の組み込みのリトライは有効にしない(設定しなければ無効)

#### エラーが起きたときの流れ

1. 呼び出し側が「自分で処理する」と渡したエラーコード(`handledCodes`。省略できる)なら、ダイアログを出さずにすぐ返す
2. カスタムエラーハンドラーに登録されたコードなら、そのハンドラーが処理する
3. それ以外は、共通エラーハンドラー(`ICommonErrorHandler`)が処理する(3.5の action の表のとおり)

- 予約コード(1〜999)は1と2を通らず、必ず共通エラーハンドラーが処理する。画面ごとにメンテナンスなどの扱いが変わらないようにするため
- ハンドラーは「再送する / 諦める」を返す。再送なら、順番待ちを持ったまま同じRequestIDで送り直す。諦めたら、呼び出し側に失敗を返す
- **エラーハンドラーの呼び出しは1つずつにする**(ダイアログは同時に1つだけ)。順番待ちに並ばないRPC(ポーリングなど)は同時にエラーになり得るため。後のエラーは前のダイアログが閉じるまで待ち、前のダイアログでタイトル・ホームへ戻った場合は、ダイアログを出さずに失敗として終わらせる

| action | ダイアログの後 | 呼び出し側に返るもの |
|---|---|---|
| Retry | 「再試行」なら同じRequestIDで再送する。「タイトルへ」ならタイトルに戻る | 再送で成功すれば成功。それ以外は失敗 |
| Stay | 画面に留まる | 失敗 |
| Home / Title | 画面を遷移する。順番待ちに残っているリクエストは送らずに破棄する | 失敗(続きの処理は動かさない) |
| Update | ストアのページを開く。アプリは先へ進めない | 失敗 |

#### 画面ごとの個別の処理

- `handledCodes` で渡したコードは、ダイアログを出さずに返る。例: タイトル画面が、サインインの「プレイヤー未作成」を受け取ってサインアップへ進む
- 返した時点で順番待ちは解放される。もう一度呼ぶと新しいRequestIDになるが、エラーのレスポンスはサーバーに保存されないので問題ない
- どの画面から呼んでも同じ表示にしたいコード(例: ガチャ期間外の専用のポップアップ)は、カスタムエラーハンドラーで処理する

#### カスタムエラーハンドラー

```csharp
[CustomErrorHandler(ErrorCode.GachaClosed, ErrorCode.GachaTicketShortage)]
public sealed class GachaErrorHandler : ICustomErrorHandler
{
    public UniTask<ErrorDecision> HandleAsync(ApiError error) { ... }
}
```

- 機能ごとに複数作れる。担当するエラーコードは属性で書く。1つのエラーに対して動くハンドラーは1つだけ
- **担当コードの重複と、予約コードの指定は、基盤のRoslynアナライザーでコンパイルエラーにする**。アプリは属性を付けたハンドラーを書いてDIに登録するだけで、重複は書いた時点で気づける
- `ApiCore` も起動時に同じチェックをする(アナライザーが効いていないアセンブリがあっても気づけるように)
- v1では、カスタムエラーハンドラーを置くアセンブリを1つにする(アナライザーはコンパイル中のアセンブリしか見渡せない)
- `ApiCore` と同じルートのDIスコープに登録する(子のスコープの登録は `ApiCore` から見えない)

#### 共通エラーハンドラー

- **基盤が既定の実装(`DefaultCommonErrorHandler`)を持つ**。action → ボタン → 挙動はどのアプリでも同じなので、メンテナンスやBANの扱いがアプリごとにずれない。Titleのときのセッションの破棄も、この中で行う
- アプリは、ダイアログの見た目と画面遷移だけを実装する。カスタムエラーハンドラーも同じものを使い回せる

```csharp
public interface IErrorDialogView   // ダイアログの見た目(prefab)。押されたボタンを返す
{
    UniTask<ErrorDialogButton> ShowAsync(ApiError error, ErrorDialogButtons buttons);
}

public interface IErrorNavigator    // 画面遷移
{
    UniTask GoToTitleAsync();
    UniTask GoToHomeAsync();
    UniTask OpenStoreAsync();
}
```

- 既定の動きが合わないアプリは、`ICommonErrorHandler` ごと差し替えられる
- `ApiCore` は結果(再送する / 諦める)だけを受け取り、ダイアログの表示や画面遷移には関わらない
- 基盤のエラー表示はすべてダイアログ(閉じるのにユーザーの操作が要る)にし、トースター(自動で消える表示)は持たない。トースターにしたいアプリは、カスタムエラーハンドラーで出し、すぐ「諦める」を返す(予約コードは担当できないので、通信エラーやメンテナンスがトースターになることは無い)。トースターで済むのは、action が Stay の業務エラー(例: ジェム不足)だけ。一時的な失敗は、再試行を選んでもらうためにダイアログが要る

### 5.5 認証とアカウント情報

パッケージの Auth(Unity に依存しない .NET のコード。名前は 5.1 の注記のとおり、作るときに決める)が持つ。`ApiCore` とは、トークンを渡す interface(例: `ITokenProvider`)でつなぐ。保存先のパスとバンドルIDは、Unity 用の層が渡す。

#### デバイス登録とトークン

- **デバイス登録とトークンの発行は自動で行う**(`ApiCore` がトークンを求めたときに、Auth が行う)。`device_id` と `secret_key` が無ければデバイス登録し、有効なトークンが無ければ(または有効期限が近ければ。Atlasは残り5分)発行してから、呼び出されたAPIを送る。アプリは、タイトル画面でサインインを呼ぶだけでよい
- デバイス登録も、トークンの発行も **single-flight** にする(同時に走ると、`device_id` が2つ発行されたり、互いのトークンを無効にし合ったりするため)
- デバイス登録は、アプリが最初にAPIを呼んだ時点で行われる。利用規約への同意などを先に済ませたい場合は、その後で最初のAPIを呼ぶ
- それでも `UNAUTHENTICATED` が返ったら、再認証して**1回だけ**同じRequestIDで送り直す。サーバーが処理を始める前に弾いたものなので、二重実行にならない
- 起動時の流れ: デバイス登録(未登録なら) → トークンの発行 → サインイン →(「プレイヤー未作成」なら)サインアップ

#### 保存の仕組み

端末に保存するのは `device_id`、`secret_key`、`player_id`。保存の仕組みは基盤が用意し、アプリが差し替えられるようにする。

```csharp
public interface IAccountStore
{
    UniTask<AccountData?> LoadAsync(CancellationToken ct);    // 未登録ならnull
    UniTask SaveAsync(AccountData data, CancellationToken ct);
    UniTask ClearAsync(CancellationToken ct);
}
```

- 非同期にするのは、OSの安全な保存領域を使う実装に差し替えることも考えるため。`ApiCore` は最初の読み込みでメモリに持ち、毎回は読まない

| タイミング | 書く人 | 内容 |
|---|---|---|
| デバイス登録の後 | Auth | `device_id`、`secret_key` |
| サインアップ・サインインの後 | アプリ(呼び出し側が結果を受け取った後) | `player_id` を足す |
| アカウントの引き継ぎ・データ削除 | アプリ | 入れ替える、または消す。その後 `ApiCore` のセッションを破棄する(例: `ResetSession()`)。次の呼び出しで読み直し、トークンを取り直す |

#### 既定の実装(`EncryptedFileAccountStore`)

自作パッケージ Supplement の暗号化ファイル保存を、**パッケージとしては依存せず、コードをコピーして**作り変える。パッケージ(Unity に依存しない)に置くので、壊れたファイルの扱いも含めて EditMode テストでテストできる。

```
[識別子 | バージョン | salt | 暗号文 | HMAC]
                              └ AES で暗号化した AccountData(Protobuf のバイナリ)
```

| 部分 | 基盤での実装 |
|---|---|
| データの形 | `AccountData` をパッケージの `.proto` で定義し、**Protobuf のバイナリ**にする。項目を後から足しても、古いファイルを読める |
| 暗号化 | AES(CBC)。保存のたびにランダムなsaltを作り、PBKDF2で鍵とIVを作る(Supplementのまま) |
| 改ざん・破損の検出 | **HMAC**を付け(Supplementのフォーマットからバージョンを上げる)、復号する前に確かめる。CBCでは途中が壊れても復号が成功することがあり、`secret_key` だけが黙って壊れ得るため |
| 鍵の元 | パッケージの中で、バンドルID(Unity用の層が渡す)から作る。アプリは渡さない(コンストラクタで上書きはできる)。鍵の元はファイルに含めない |
| 書き込み | 一時ファイルに書いてから置き換える(Supplementの `SafeFile` のまま) |
| 保存先 | パッケージ用に固定する(アプリのセーブデータと共有しない)。パスは Unity用の層が渡す |
| 公開範囲 | 名前空間をパッケージのものに変え、`internal` にする(アプリがSupplementも使っていても名前が衝突しないように) |

- **読み込みのルール**: ファイルが無ければ未登録(`null`)。ファイルがあるのに読めない(HMACが合わない、復号できない、値が空)場合は専用の例外にし、**新しいアカウントを勝手に作らない**(元のデータに戻れなくなるため)。問い合わせや引き継ぎに誘導する
- 鍵の元がアプリの中にある以上、暗号化は難読化の位置づけ(他のアプリからはOSのサンドボックスで守られる)。本当に守りたいアプリは、iOSのKeychainやAndroidのKeystoreを使う実装に差し替える
- PBKDF2は本来は要らないが、コストは無視できるので、動作確認済みのコードを変える量を減らすために残す
- 注意(v1ではドキュメントに書くだけ): 保存したファイルはOSのバックアップに含まれることがある。別の端末に戻すと、2台が同じ `device_id` を持ち、認証のたびに互いのトークンを無効にし合う。バックアップから外すかはアプリに任せる

### 5.6 共通レスポンス項目の反映

`ApiCore` は、成功したら呼び出し側に返す前に、共通レスポンス項目(3.9)をアプリ側の反映処理に渡す。順番は playerDiff → バッジ → 通知。

```csharp
// PlayerDiff などはアプリ側の型なので、基盤はレスポンスごと渡す
public interface IPlayerDiffApplier   { UniTask ApplyAsync(IMessage response); }   // player_diff を Repository に反映する
public interface IBadgeApplier        { UniTask ApplyAsync(IMessage response); }   // badges でバッジの状態を更新する(含まれない種類は触らない)
public interface INotificationApplier { UniTask ApplyAsync(IMessage response); }   // notifications を待ち行列に積む
```

- **反映処理は、`.proto` でそのフィールドを使っているときだけ必須にする**。使っているのに登録を忘れたら、起動時にエラーにする(プラグインが生成した一覧で確かめる。3.4)
- **順番待ちの中で行う**。次のAPIが送られる時点で、必ず反映が終わっている
- **反映処理でやるのはデータの更新だけ**にし、ダイアログや画面遷移は書かない。通知を出すタイミングはアプリの画面が決める(反映処理の中で出すと、ガチャの演出の途中に割り込む)
- `IPlayerDiffApplier` の中では、リソースごとの反映を並列(`UniTask.WhenAll`)に呼ぶ。反映の順序に依存する作りにはしない
- **APIごとの処理は、呼び出し側が `await` で結果を受け取った後に書く**(例: サインインの後に `player_id` を保存する、サインインの全件を置き換える)。全APIに共通の処理が必要になったら、共通レスポンス項目として足す
- 変化前の値が必要な演出(レベルアップ前のレベルなど)は、呼び出す前に値を取っておく(返った時点で反映が終わっているため)
- サーバーで計算したバッジと、クライアントで判定したバッジ(3.9)は、アプリ側(`BadgeService` など)で種類ごとにまとめて画面に見せる

当面の実装:

- 反映処理は `IPlayerDiffApplier` だけを用意し、`ApiCore` のコンストラクタで受け取る(省略できる)。バッジ・通知は、サーバー側と合わせて足す
- `ApiCore` は、通信の `try` の外で反映処理を呼ぶ。通信のエラーと同じ扱いにすると、エラーハンドラーが再試行を選んだときに送り直してしまうため
- アプリの `PlayerDiffApplier` は、レスポンスの Descriptor から規約のフィールド名 `player_diff` を探して読み、`PlayerRepository` に反映する

#### 反映処理で例外が起きたとき

反映処理の例外は、アプリ側の実装のバグ。サーバー側の処理は成功しているので、再送はしない。

- 順番待ちと通信中の入力ブロックを**必ず解除する**(`finally`)。解除しないと、以降のAPIがすべて待たされ、フリーズしたように見える
- 例外の内容(どの反映処理で、どのレスポンスのときか)をスタックトレース付きでログに出し、`CLIENT_ERROR` として共通エラーハンドラーに渡す。タイトルに戻ってサインインで全件を取り直し、サーバーとの食い違いを解消する
- 呼び出し側には失敗を返し、続きの処理は動かさない
- 開発ビルドとリリースビルドで動きは変えない。開発中に必ず気づけるように、全APIの反映処理をテストで通す(5.8)

### 5.7 接続設定

| 項目 | 既定値 | 補足 |
|---|---|---|
| deadline(1回の通信全体の制限時間) | 15秒 | 全RPC共通。アプリ側から上書きできる。v1ではRPCごとの上書きはしない |
| `ConnectTimeout`(つながるまでの制限時間) | 10秒 | YetAnotherHttpHandler は既定では無効。アプリ側から上書きできる |
| 受信サイズの上限(`MaxReceiveMessageSize`) | 4MB(Grpc.Net.Client の既定) | 変更できるようにする。開発中は、レスポンスが上限の半分を超えたら警告ログを出す |
| HTTP/2 の keepalive | 無効(既定のまま) | 問題が出たら見直す |

- deadline は接続の部分も含むので `ConnectTimeout` は必須ではないが、つながらないときに早めに諦め、裏で接続を試み続けるのを止めるために設定する
- どちらのタイムアウトも `NETWORK_ERROR` になり、再試行のダイアログが出る。再試行は同じRequestIDなので安全。サーバーも同じ deadline で処理を打ち切ってロールバックする(4.2)
- 受信サイズの上限を超えると `UNKNOWN_ERROR` になり、サインインの全件データが大きくなると誰もサインインできなくなる。警告で早めに気づけるようにする
- **接続先**: ローカルは `http://` で、YetAnotherHttpHandler の `Http2Only = true` を付ける(h2c。指定しないとHTTP/1になる)。開発・本番のサーバーは `https://` で、公的な証明書(AWSならACM)を使う。接続先のURLはアプリ側から `ApiCore` に渡し、環境の切り替え方はアプリ側に任せる
- **実機からローカルのサーバーにつなぐとき**: WindowsのPCで立てたサーバーに、LAN内のIPアドレスで接続する(ファイアウォールでポートを開ける)。iOS・Androidの暗号化なしの通信の制限が効くかは、初めて実機で試すときに確かめる
- サーバーの前段のロードバランサーは、HTTP/2 を通せる必要がある(AWSのALBは対応、App Runnerは非対応)

### 5.8 サーバーが無いときの差し替えとテスト

#### FakeCallInvoker

- 基盤は、生成クライアントが実際に通信する部分(`CallInvoker`)の代わりに、RPCごとに登録した処理を呼ぶ **`FakeCallInvoker`** を用意する
- アプリは、gRPCサービスごとに偽のサーバー(例: `FakeGachaServer`)を書く。状態を持たせて(ジェムの残りなど)、本物と同じ型のレスポンス(共通レスポンス項目も含む)やエラーを返せる。状態を持つので「フェイク」と呼ぶ
- ラッパー、`ApiCore`、反映処理、エラー処理は本物と同じように動くので、サーバーができたらDIの登録を切り替えるだけで済む。クライアントがサーバーより先に作業するときも、`.proto` で形を決めてから偽のサーバーで進める
- **ラッパーの interface は生成しない**(interface でモックを作ると `ApiCore` を通らず、本物と動きが変わる)
- Connection 層やApplication 層にモックを置くかは、アプリの設計なので基盤では決めない

#### テスト

アプリが書くのは偽のサーバーとサンプルのリクエストだけにし、テストの中身は基盤が用意する。

- アプリ: gRPCサービスごとの偽のサーバーに、RPCごとのレスポンスとサンプルのリクエストを登録する
- 基盤: 汎用のテストを2つ用意する(テストクラスを継承するだけで使える)
  1. 全RPCに偽のレスポンスが登録されているか(全RPCの一覧はプラグインが生成する。3.4)
  2. 全RPCのサンプルのリクエストを、ラッパー → `ApiCore` → 反映処理の順に通し、`CLIENT_ERROR` が起きないか
- 基盤: テスト用の部品(押されたボタンを記録するだけのダイアログ、メモリ上に保存するだけの `IAccountStore` など)
- 新しいAPIも、偽のサーバーに登録すれば自動でテストされ、登録を忘れればテストが失敗する。特定のデータのときだけ起きるバグはすり抜け得るが、その場合も `CLIENT_ERROR` でタイトルに戻って全件を取り直す

---

## 6. 開発運用 【確定】

### 6.1 `.proto` の管理

- **buf** を使う。`buf lint`(書き方)、`buf format`(整形)、`buf breaking`(後方互換を壊す変更の検出)
- C#の生成は `buf generate` で行う(protoc組み込みのC#生成、grpc_csharp_plugin、ラッパー生成のプラグイン)。Rustは `build.rs` で生成する(protox で `.proto` を読み込み、tonic-prost-build で生成する。protoc は要らない)
- 一度使ったフィールド番号・名前は再利用しない(消したら `reserved` にする)

### 6.2 デバッグ

- **Postman** を使う。サーバーは開発環境のときだけ gRPC リフレクションを有効にする(`tonic-reflection`)ので、`.proto` を変えるたびにインポートし直さなくてよい
- RequestIDや認証トークンは、Postman の Metadata タブで設定して送る
- 手順をコマンドで残したいときは grpcurl も使える。サーバーを作る前に応答の形を確かめたいときは、Postman のモックサーバーも使える
- Unity の EditMode テストは、`ApiCore` ができるまで、テスト専用の Interceptor(`LoggingInterceptor`)でリクエストとレスポンスを Console に出す。5.3 の方針の例外で、`ApiCore` ができたらテストを `ApiCore` 経由にして消す

### 6.3 テスト

- サーバーの結合テストは、tonic が生成するクライアントで書く(実際のクライアントと同じ経路を通る)。基盤の動作として次を確かめる
  - 同じRequestIDで2回送ると、2回目は保存済みのレスポンスが返り、業務データは1回分しか変わらない
  - 同じRequestIDで別のメソッドを呼ぶと、エラーになる
  - `NO_SIDE_EFFECTS`・`IDEMPOTENT` のRPCは、レスポンスが保存されない
  - `skip_auth` のサービスはトークンなしで呼べ、それ以外は `UNAUTHENTICATED` になる
- `IDEMPOTENT` と宣言したAPIは、その結合テストで「2回呼んでも状態が同じ」を確かめる
- クライアントでは、全エラーコードに `default_action` が付いているか、全コードの文言キーが文言データにあるかも確かめる
- テンプレートの `error_code.proto` の予約コードが、パッケージの持つ一覧と一致しているかを確かめる
- パッケージのリポジトリでは、クライアントの基盤のテスト(Unity の EditMode テスト)と、認証・状態の判定のテストを回す。テンプレートのリポジトリでは、サーバーの基盤の結合テストを回す。テンプレートとサンプルは、パッケージのバージョンを上げたときに結合テストで確かめる

| タイミング | 実行するもの |
|---|---|
| コードを書いている最中 | Roslynアナライザー(カスタムエラーハンドラーの担当コード。5.4) |
| `.proto` を変えたとき | `buf lint` / `buf breaking`(`buf generate` の前) |
| プッシュする前(手元) | `make test` の1コマンドで、`cargo test`(結合テスト。docker-compose の MySQL を使う)と、Unity の EditMode テスト(クライアントの本体と Unity用の層) |
| プルリクエスト(CI) | サーバーのテストと buf を GitHub Actions で実行する(MySQLもコンテナで立てる)。Unity のテストは、CIでのライセンス認証の設定に手間がかかるので、v1では手元だけ(クライアントの本体も含む) |

### 6.4 パッケージとテンプレートを同時に直すとき

> リポジトリを分けた場合の手順。当面は1つのリポジトリ・1つの Cargo ワークスペースなので、この手順は要らない(2.6)。

- Rust: テンプレートの `Cargo.toml` の `[patch]` で、`unigrpckit-auth`・`unigrpckit-status` を手元の `unigrpckit` のフォルダに向ける
- Unity: `Packages/manifest.json` を、一時的に手元の `unigrpckit/unity` を `file:` で参照する形にする
- 直し終わったら `unigrpckit` にタグを打ち、テンプレートの参照をそのタグに戻す

---

## 7. 付録: Atlasの乗せ換え

Atlasを乗せ換える(1.4)ときのメモ。テンプレートの設計には関わらない。

### 7.1 置き換わるもの

| Atlasの今の仕組み | gRPCでの置き換え先 |
|---|---|
| api-codegen が生成する `ApiRequest`(UnityWebRequestでJSONを送受信) | `GrpcChannel` + YetAnotherHttpHandler、生成されたメッセージ型 |
| api-codegen が生成する `XxxApiClient` | 生成されたクライアントと、ラッパー(5.2) |
| `Authorization` ヘッダー、ログ出力 | `ApiCore`(5.3) |
| `ApiException`(HTTPステータスと本文) | エラーコードと action、`ApiResult`(5.4) |
| HTTPステータス5種類のエラー(`AppError`)と、クライアントのステータスでの分岐(`401` で再認証、`404` でサインアップへ) | 業務ごとのエラーコード(3.5)。「プレイヤー未作成」は `handledCodes` で受け取る |
| `AccessTokenRefresher`(期限前の再認証、`401` 時の再送、single-flight) | `ApiCore`(5.5) |
| `PlayerDiffApplier`、サーバーでの差分の手組み(`build_full_player_diff` など) | 反映処理(5.6)、差分コレクター(4.4) |

- BattleServer(C#)からの内部API(`/internal/battle/*`)は、乗せ換えの段階ではRESTのまま残す(移すかは乗せ換え時に判断する)
- Atlasのスカウトはテンプレートのサンプルにしない。パチモン・技・バナーなどのマスタに依存し、基盤と関係ない仕組みまでテンプレートに入ってしまうため

### 7.2 クライアントの今の構成の課題

今の呼び出し経路: `Presenter → XxxService → IXxxConnection → XxxApiClient → ApiRequest`

| # | 課題 | 基盤での扱い |
|---|---|---|
| 1 | Serviceが素通りになっている(1回の通信に4層を経由する) | アプリの設計として、サンプルを作るときに決める |
| 2 | Repositoryの更新が見えない場所(Connection内)で起きる | 「playerDiffは必ず `ApiCore` が反映する」にまとめる(5.6) |
| 3 | 共通処理(トークン更新、playerDiffの反映、DTOの詰め替え)を毎回手で書いている | `ApiCore` と生成したラッパーで解消する(5.2、5.3) |
| 4 | Connectionの分け方がサーバーのURLに引きずられている | アプリの設計として、サンプルを作るときに決める |
| 5 | Mock/Realの方針と実装が食い違っている | `FakeCallInvoker` で切り替えをDIの登録1か所にまとめる(5.8) |
| 6 | 生成クライアントをConnectionの中で生成しており、通信部分だけを差し替えられない | アプリの設計として、サンプルを作るときに決める |

---

## 8. 採らなかった案

| 項目 | 採らなかった案 | 理由 | 節 |
|---|---|---|---|
| サーバーの分け方 | マイクロサービスに分ける | 1リクエスト1トランザクションの前提が崩れ、補償処理(Saga)やサービスごとの冪等性が要る。開発者1人では利点が無い | 2.3 |
| サンプルの置き場所 | テンプレートのアプリの雛形に置き、アプリを作るときに書き換える | サンプルが手元に残らず、基盤側がサンプルを直すと取り込みで衝突する | 1.3 |
| サンプルの置き場所 | テンプレートの中の `samples/` に置く | 基盤を取り込んでアプリを作る手順を試せない | 1.3 |
| サンプルの機能 | Atlasのチャット機能 | 業務エラーがほとんど無く、所持データを変えないので playerDiff を試せない | 1.4 |
| リポジトリ名 | 後ろに通信方式(gRPC)を付ける | 増えるのはサーバーの言語。gRPC は設計全体の前提 | 2.2 |
| リポジトリの運用 | GitHubのフォーク、「Use this template」 | 自分のリポジトリは同じアカウントにフォークできない。テンプレート機能は複製元との履歴のつながりが無くなる | 2.1 |
| パッケージにする範囲 | 基盤をすべてパッケージにする | `with_transaction()` やコレクターがアプリの型を知らずに動くように、trait とジェネリクスの設計が要る。アプリが `error_code.proto` にコードを書き足せない。アプリ側で直す可能性がある部分まで固定される | 2.1 |
| テンプレートの構成 | 基盤を `foundation/`、アプリを `app/` に分け、Unity プロジェクトを `app/client` に置く | アプリを作るときのいつもの形(`Client/`・`Server/`)と違う |
| クライアントの基盤 | テンプレートに置き、Unity プロジェクトから `file:` で参照する | clone したアプリにソースのコピーが付いてきて、アプリが直接直せてしまう。テンプレートとパッケージの2つの更新のやり方が混ざる |
| クライアントの基盤 | クライアントの基盤だけで1つのリポジトリにする | サーバーの認証と `device.proto` を共有し、プラグインと `ApiCore` も組で動くので、同じリポジトリ・同じタグで出す方が食い違わない |
| 名前 | パッケージとテンプレートで頭をそろえる(GameGrpcKit + GameGrpcServer) | パッケージの中心は Unity の通信基盤なので「Uni」を付けた |
| パッケージのリポジトリ | Unity の通信基盤・サーバーのパッケージ・proto ツールで、リポジトリを分ける | プラグインと `ApiCore`、`device.proto` と両側の認証が組で動くので、分けると1つの変更で複数のリポジトリにタグを打ち、組み合わせの対応表を管理することになる。使う側は UPM の `?path=`、クレート名、dotnet tool で必要なものだけを取れる |
| パッケージとテンプレート | 同じリポジトリで管理する | clone したアプリにパッケージのソースのコピーが付いてくる。両方を同時に直すのは、手元のパスへの差し替えで足りる(6.4) |
| パッケージにする範囲 | パッケージにしない(すべて clone して取り込む) | アプリごとに変えない認証まで、アプリが直接触れる。サーバーとクライアントの認証のバージョンがずれ得る | 2.1 |
| 通信方式 | v1からストリーミングを使う | モバイルの切断時の再接続・取りこぼし処理、サーバー複数台での配信の仕組み(Pub/Sub等)、長時間接続を切らないロードバランサーの設定が要る | 3.2 |
| コード生成 | RoslynのSource Generator | Unityが対応するRoslynのバージョンに縛られ、デバッグしにくい。サービスごとの設定をC#の属性に書くことになり、サーバーと別々に管理することになる | 3.4 |
| コード生成 | Unityのエディタ拡張でリフレクションして生成 | `.proto` でRPC名を変えるとコンパイルが通らなくなり、生成スクリプト自体が動かなくなる | 3.4 |
| コード生成 | プラグインをRustで書く | prost がカスタムオプションを読めず、`prost-reflect` が別に要る | 3.4 |
| 冪等性の指定 | 独自のカスタムオプション、サービスを分けて表す | 標準の `idempotency_level` で表せる | 3.6 |
| エラーの返し方 | details に独自のメッセージを定義する | 標準の `ErrorInfo` なら Rust・C# ともライブラリの型がそのまま使え、引数も `metadata` で運べる | 3.5 |
| エラー時の挙動 | エラーマスター(code / 文言キー / action)で管理する(実務の方式) | テンプレートが master-data-pipeline に依存し、マスターを読む前のエラーを扱えない。将来差し替えられる形にはしておく(5.4) | 3.5 |
| エラー時の挙動 | サーバーが遷移先の画面を指示する | サーバーがクライアントの画面構成を知ることになる。action の上書きだけを残した | 3.5 |
| エラー時の挙動 | C#に「code → action」の表を手書きする、プラグインで表を生成する | コードを足すたびに表も足す必要がある / プラグインの機能が増える | 3.5 |
| 冪等性の保存 | 一定期間すべてのレスポンスを保存する(Stripe方式) | 保存量が増え、期限切れの掃除が要る。ゲームは1つずつ送るので最新1件で足りる | 4.3 |
| 冪等性 | RequestIDをリクエストメッセージのフィールドに持つ(AIP-155) | 全APIに一律に適用するので、メタデータに載せる | 3.6 |
| 冪等性 | RequestIDを Interceptor で採番する | 再試行のたびに別のIDになる | 5.3 |
| playerDiffの載せ方 | メタデータ(トレーラー)で返す | サイズ上限(8〜16KB程度)があり、サインインの全件が載らない(アプリボットは実際に上限を超えた) | 3.9 |
| playerDiffの載せ方 | 全レスポンスを共通の入れ物(`{ PlayerDiff diff; Any body; }`)で包む | APIごとの型が消える | 3.9 |
| playerDiffの値 | 変化量(ジェム-100)で持つ | 2回反映すると結果が変わり、保存済みのレスポンスの再送と両立しない | 3.9 |
| 差分の集め方 | ハンドラーが手で組み立てる(Atlasの方式) | 副作用で変わったものを入れ忘れ得る | 4.4 |
| 差分の集め方 | 自動生成したデータアクセス層が変更を検知する(学マスの方式) | データアクセス層まで生成する大がかりなコード生成が要る | 4.4 |
| バッジ | 種類をマスターで持つ | 種類を足すときはサーバーに判定処理を書くので、マスターの利点が無い | 3.9 |
| バッジ | 「どのAPIでどの種類を返すか」の表を持つ(実務の方式) | 書き込み関数が記録する形なら、表を手で保守しなくてよい | 4.4 |
| サーバーの共通処理 | 冪等性・トランザクション・コレクターを tower Layer にする | レスポンスの型に依存し、trailer やFuture型の制約にぶつかる。`Transaction` が `Sync` でなく Extensions に載らない | 4.2 |
| サーバーの共通処理 | 認証を `tonic::Interceptor` で書く | 同期関数専用で、DBに問い合わせられない | 4.2 |
| サーバーの状態 | メンテナンスなどの状態をDBに置く | メンテナンス中にDBを止めると判定できない | 4.6 |
| クライアントの本体 | 本体のAPIを `ValueTask` にし、Unity用の層で UniTask に変える | Unity用の層が厚くなる | 5.1 |
| クライアントの共通処理 | Interceptor で書く | `await` するには `AsyncUnaryCall` を作り直す必要があり、重ねると例外が `AggregateException` に包まれる。同じRequestIDでの再送も回りくどい。ラッパーを必ず通るので `ApiCore` で足りる | 5.3 |
| ラッパーの形 | APIごとに1クラス(実務のRESTの `ApiBase` の形) | 中身が1行のクラスがAPIの数だけでき、Serviceクラスの依存も増える | 5.2 |
| ラッパーの形 | ラッパーを作らず、`ApiCore` を直接呼ぶ | gRPCの型が隠れず、呼び出し側にラムダを書くことになる | 5.2 |
| ラッパーの形 | メソッド名を `SendAsync` にそろえてオーバーロードする | 補完でAPIを区別しにくく、ログやスタックトレースにAPI名が出ない | 5.2 |
| ラッパー | 呼び出し側がキャンセルできる(`CancellationToken` を受け取る) | 送信後に打ち切ると、サーバーの処理結果がクライアントに反映されない | 5.2 |
| リトライ | 通信層はエラーを必ず呼び出し側に返す | 「再試行は同じRequestID」「成功するか諦めるまで次を送らない」と両立しない | 5.4 |
| リトライ | 自動リトライ、Grpc.Net.Client の組み込みリトライ | 再送するかはユーザーが選ぶ | 5.4 |
| エラーの返し方 | 呼び出し側に例外を投げる | 失敗の多くは「抜けるだけ」で、毎回 `try/catch` を書くのは大げさ。catch し忘れると追いにくい | 5.4 |
| カスタムエラーハンドラー | switch式で振り分けを1か所に書く | ハンドラーを足すたびに振り分けも書き換える必要があり、量産に向かない | 5.4 |
| カスタムエラーハンドラー | 担当コードをプロパティで持ち、起動時にだけチェックする | 気づくのが起動時・テスト時になる | 5.4 |
| エラー表示 | 基盤にトースターを持つ | 通信エラーが勝手に消えると、ユーザーは何も起きないと感じ、再試行もできない | 5.4 |
| 共通レスポンス項目の反映 | 全APIで汎用に使える成功後のフック(`IApiResponseHook`) | APIごとの処理は呼び出し側に書けば足りる。共通の処理は共通レスポンス項目として足す | 5.6 |
| 共通レスポンス項目の反映 | アプリのServiceクラスがそれぞれ反映する(実務・Atlasの方式) | 呼び忘れがたびたび起きた | 5.6 |
| アカウント情報の保存 | JsonUtility で JSON にする | Unity に依存し、本体(Core)に置けない | 5.5 |
| アカウント情報の保存 | BinaryWriter で手書き、MessagePack-CSharp | 項目を足すときにバージョンを自分で管理する / 依存が増える | 5.5 |
| アカウント情報の保存 | `BinaryFormatter` | 安全上の問題があり、.NET では廃止の方向 | 5.5 |
| アカウント情報の保存 | Supplement にパッケージとして依存する | 基盤の用途に合わせて作り変えるため、コードをコピーする | 5.5 |
| サーバーが無いときの差し替え | ラッパーの interface を生成してモックを作る | `ApiCore` を通らず、本物と動きが変わる | 5.8 |
| 接続設定 | 開発サーバーで証明書の検証を飛ばす(`SkipCertificateVerification`) | 開発サーバーも公的な証明書を使う | 5.7 |

---

## 9. 未確定・保留

| # | 項目 | 状態 | 節 |
|---|---|---|---|
| 1 | クレートの細かい分け方(テンプレートの基盤のクレートの分け方、ドメインのクレートが生成コードに直接依存するか) | 実装時に決める | 2.4, 4.1 |
| 2 | テンプレートの `Client/`・`Server/` の雛形の中身 | サーバー側は、起動・デバイス登録・トークンの発行まで実装済み(2.6)。Unity 側は作るときに決める | 2.4 |
| 3 | テンプレートの buf から、パッケージの `.proto`(カスタムオプションの定義)を参照する方法(BSR に置く、コピーする、など) | 当面は手元のパスで参照する(2.6)。リポジトリを分けるときに決め直す | 2.4, 3.4 |
| 4 | 細かい名前(ラッパーのクラス名の規則、`x-request-id`・`authorization` 以外のヘッダー名、`MAINTENANCE` の `metadata` のキー名、パッケージの中のフォルダ名) | 実装時に決める | 3.8, 5.2 |
| 5 | サインインの全件の置き換えと、`IPlayerDiffApplier` による自動の反映の区別(フィールド名を分けるなど) | 実装時に決める | 3.9, 5.6 |
| 6 | サーバーでも `skip_auth` を読むか(`prost-reflect`) | v1の後に判断する | 4.5 |
| 7 | ソフトメンテ(IPの許可リスト)と、設定の置き場所をAWSの設定サービスにすること | 一般のユーザーに公開する前に足す | 4.6 |
| 8 | `player_id` の許可リスト、通常メンテと緊急メンテの出し分け、管理画面 | 運用が大きくなってから | 4.6 |
| 9 | 「No Engine References」の asmdef から UniTask を参照できるか、UPM と csproj で同じソースを管理する方法 | 参照できる(`AsUniTask()` だけ使えない。5.1)。csproj では管理せず、asmdef だけで持つ | 5.1 |
| 10 | サンプルのガチャの排出物を、小さなマスタにするかコード内の固定テーブルにするか | コード内の固定テーブルにする(`gacha::model::ITEMS`)。基盤を試すのが目的なので、マスタの仕組みは持たない | 1.3 |
| 11 | サンプルにサインアップを入れるか(サインイン時にプレイヤーを自動で作る形にするか) | 入れる。サインアップは何も受け取らない。サインインはプレイヤーとジェムを返し、未作成なら `PLAYER_NOT_FOUND`(`app/v1/player.proto`) | 1.3 |
| 12 | クライアントのサンプルの画面の数と中身 | サンプルのリポジトリを作るときに決める | 1.3 |
| 13 | AtlasのREST版を残すブランチの名前・作る時期 | 乗せ換えを始めるときに決める | 1.4 |
| 14 | コンテナ化とAWSへのデプロイの進め方 | gRPCへの移行とAPIの実装が終わった後 | — |
| 15 | パッケージの Rust 側で、`device.proto` から生成したコードをコミットするか、使う側の `build.rs` で生成するか(使う側に protoc が要るかどうか) | `build.rs` で protox を使って生成する(protoc は要らない。2.6) | 2.4 |
| 16 | パッケージのリリースの手順(タグの付け方、変更履歴) | パッケージを作るときに決める | 2.1 |
| 17 | パッケージ・リポジトリの分け方(分けるか、いつ・どの単位で分けるか、名前) | 実装を進めて形が固まってから検討する。それまでは 2.6 の方針で進める | 2.1〜2.4, 2.6 |

---

## 10. 参考資料

### 全体

- CEDEC2025「『学園アイドルマスター』のバックエンドを支える基盤システムの仕組み」(QualiArts) — 「学マス資料」。[セッション概要](https://cedec.cesa.or.jp/2025/timetable/detail/s67a5b1155ef14/)、[講演レポート](https://www.gamebusiness.jp/article/2025/08/15/24859.html)。レスポンスキャッシュ(p.51)、共通レスポンス(p.50)、Protocol Buffersへの定義の集約
- CEDEC2026「次世代デジタルカードゲームのサーバー設計と運用 〜『Shadowverse: Worlds Beyond』の舞台裏〜」(Cygames / Cysharp) — 「シャドバWB資料」。[スライド](https://speakerdeck.com/cygames/cygames_202607_cedec2026_06)。ストリーミングを複数台で配る仕組み(p.71〜72)
- 実務のRESTの通信基盤(非公開) — キューで1件ずつ送り、エラーのダイアログを通信処理の中で待ち、同じRequestIDで再送していた。エラー処理は「APIごとのハンドラー → エラーマスタ → 共通エラー」の3段階

### リポジトリ構成(2章)

- [Microservice Premium](https://martinfowler.com/bliki/MicroservicePremium.html)(Martin Fowler) — モノリスで扱えないほど複雑になるまでは、マイクロサービスのコストの方が上回る
- [Deconstructing the Monolith](https://shopify.engineering/deconstructing-monolith-designing-software-maximizes-developer-productivity)(Shopify Engineering) — デプロイの単位を増やさずにモジュール性を上げる
- [『学園アイドルマスター』を支えるサーバーシステムとは？【CEDEC2025】](https://www.inside-games.jp/article/2025/08/15/170552.html)(インサイド) — APIサーバーと管理画面のサーバーを別々の Cloud Run のサービスにしている
- [grpc/grpc-go](https://github.com/grpc/grpc-go)、[grpc/grpc-rust](https://github.com/grpc/grpc-rust)、[connectrpc/connect-go](https://github.com/connectrpc/connect-go) — 言語ごとにリポジトリを分ける例

### API定義(3章)

- コード生成
  - [高速で統一的な自動生成ツールをprotocプラグインとして実装した話](https://speakerdeck.com/qualiarts/gao-su-detong-de-nazi-dong-sheng-cheng-turuwoprotocpuraguintositeshi-zhuang-sitahua)(QualiArts) — カスタムオプションで生成先を指定し、API・DBスキーマ・エンティティ等を生成する
  - [「Protocol Buffers」からGoコードを自動生成 ー独自protocプラグイン開発と「1次ソース」の設計思想](https://thinkit.co.jp/article/39506)(CyberAgent / QualiArts) — protocプラグインの作り方
  - [「IDOLY PRIDE」におけるgRPC利用とカスタマイズ](https://technote.qualiarts.jp/article/24/)(QualiArts) — カスタムオプションでAPIごとに認証・メンテナンスのチェックの有無などを切り替えている。開発用に grpc-gateway と Swagger UI を使っている
  - [Namespace Google.Protobuf.Compiler](https://cloud.google.com/dotnet/docs/reference/Google.Protobuf/3.23.0/Google.Protobuf.Compiler)(Google Cloud) — protocプラグインの入出力の型が `Google.Protobuf` に入っている
  - [Custom Options and Proto2 Extensions · tokio-rs/prost #674](https://github.com/tokio-rs/prost/issues/674) — prost はカスタムオプションに対応していない
- エラー
  - [gRPC: Error handling](https://grpc.io/docs/guides/error/) — 標準エラーモデルとリッチエラーモデル
  - [AIP-193: Errors](https://google.aip.dev/193) — `ErrorInfo` の使い方と `reason` の命名規則
  - [Error handling with gRPC on .NET](https://learn.microsoft.com/en-us/aspnet/core/grpc/error-handling)(Microsoft Learn)、[tonic-types](https://docs.rs/tonic-types/latest/tonic_types/) — C# と Rust でのリッチエラーモデル
  - [Carpe Diem「gRPCでエラー詳細を渡す方法」](https://christina04.hatenablog.com/entry/grpc-error-details)、[アルパカ三銃士「gRPC Application のエラー設計」](https://codehex.hateblo.jp/entry/2020/08/10/164630) — エラーコードの渡し方の比較
  - [Fintan「HTTP API通信で発生するエラーのハンドリング」](https://fintan-contents.github.io/mobile-app-crib-notes/react-native/santoku/decisions/adr-003-http-api-error-handling/) — メンテナンス・強制アップデートを共通処理で扱い、自動リトライせずユーザーに再試行させる
  - [Qiita「クライアントアプリでのレベル別エラー分類の提案」](https://qiita.com/yosshi4486/items/53367ba2dde668f5f038) — 「システムで回復 / ユーザーが回復 / 回復不能」の分類
- 冪等性
  - [Stripe「Designing robust and predictable APIs with idempotency」](https://stripe.com/blog/idempotency)、[brandur「Implementing Stripe-like Idempotency Keys in Postgres」](https://brandur.org/idempotency-keys) — 冪等キーの考え方と、一定期間すべて保存する方式
  - [AIP-155: Request identification](https://google.aip.dev/155) — 重複時は前回の成功レスポンスを返す
  - [MONEX ENGINEER BLOG「Web APIにおける重複エラーはエラーに非ず」](https://blog.tech-monex.com/entry/2024/12/05/133747) — 重複リクエストには初回と同じ正常レスポンスを返す
  - [gRPC でメソッドが副作用を持つ場合冪等性キーを必須にする](https://tokyo.oyasumi.club/2021/07/17/protobuf-ensure-idempotent.html) — `idempotency_level` の使い方は規定されておらず、自分たちで決める
  - [Include idempotency level in UnaryServerInfo · grpc/grpc-go #1634](https://github.com/grpc/grpc-go/pull/1634) — フレームワークは `idempotency_level` を使わない
  - [非同期処理とリトライと冪等性 - GS2 Blog](https://gs2.hatenablog.com/entry/2024/07/29/232715) — ゲーム向けバックエンドでの実装。メソッド名を一緒に保存する判断の材料
  - [Get requests and caching](https://connectrpc.com/docs/go/get-requests-and-caching/)(Connect) — `NO_SIDE_EFFECTS` の使われ方の例
  - [MethodOptions.Types.IdempotencyLevel](https://cloud.google.com/dotnet/docs/reference/Google.Protobuf/latest/Google.Protobuf.Reflection.MethodOptions.Types.IdempotencyLevel)(Google.Protobuf .NET) — C#で値を読むときの型
- 共通レスポンス項目
  - [CEDEC2019 アプリボット×Cysharpの資料](https://speakerdeck.com/n_takehata/kuraiantotong-xin-haipahuomansunatong-xin-ji-pan-falsekai-fa-tomagiconionniyoruriarutaimutong-xin-falseshi-xian) — 更新情報をトレーラーで返していたが、8KBの上限を超えた

### サーバー基盤(4章)

- [tonic issue #870: Asynchronous interceptor](https://github.com/hyperium/tonic/issues/870) — `tonic::Interceptor` は同期専用
- [sqlx::Pool::begin()](https://docs.rs/sqlx/latest/sqlx/struct.Pool.html) — `Transaction` は `Sync` ではなく、`http::Extensions` に載らない
- [sqlx::Transaction](https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html) — コミットせずに破棄されたトランザクションは自動でロールバックされる
- [Support timeouts with "grpc-timeout" header · tonic #606](https://github.com/grpc/grpc-rust/pull/606) — クライアントの deadline とサーバーのタイムアウトのうち短い方で打ち切る
- [HTTP headers and Application Load Balancers](https://docs.aws.amazon.com/elasticloadbalancing/latest/application/x-forwarded-headers.html)(AWS) — ALBは `X-Forwarded-For` の末尾に接続元のIPを足す

### クライアント基盤(5章)

- [grpc-webによる内製ゲームサーバ基盤TakashoのUnity WebGL対応](https://speakerdeck.com/dena_tech/techcon2023-session16)(DeNA) — gRPCを隠すSDKを `.proto` から生成し、共通ヘッダ・認証・エラーの分類・リトライをSDK側で行う
- [Unity用のHTTP/2(gRPC) Client、YetAnotherHttpHandlerを公開しました](https://neue.cc/2023/07/28_yetanotherhttphandler.html)(Cysharp) — UnityがHTTP/2に対応していない、C-core版のgRPCが非推奨になった
- [YetAnotherHttpHandler](https://github.com/Cysharp/YetAnotherHttpHandler)(Cysharp) — `Http2Only`、`ConnectTimeout`(既定は無効)、keepalive などの設定
- [Cysharp/UniTask](https://github.com/Cysharp/UniTask) — .NET 版は、PlayerLoop に依存する機能を除いたサブセット
- [Call gRPC services with the .NET client](https://learn.microsoft.com/en-us/aspnet/core/grpc/client)(Microsoft Learn) — 生成クライアントのメソッド名 `{RPC名}Async`
- [gRPC for .NET configuration](https://learn.microsoft.com/en-us/aspnet/core/grpc/configuration)(Microsoft Learn) — `MaxReceiveMessageSize` の既定は4MB
- [MagicOnionでの共通処理の挟み方](https://speakerdeck.com/p_a_sta/magiconiondefalsegong-tong-chu-li-falsexie-mifang)(Donuts) — Interceptorで共通処理を書くときの問題(async/await、`AggregateException`)
- [クライアントフィルター](https://cysharp.github.io/MagicOnion/ja/filter/client-filter)(MagicOnion) — `next` による非同期のパイプライン。`ApiCore` を callback で呼ぶ形の参考
- [Supplement](https://github.com/chinpanGX/Supplement) — 暗号化ファイル保存のコピー元

### 開発運用(6章)

- [Protocol Buffersで実現するWINTICKETのスキーマ駆動データ基盤](https://developers.cyberagent.co.jp/blog/archives/60672/)(CyberAgent) — Bufを採用した理由
- [PostmanでgRPCのデバッグをする](https://zenn.dev/valmet083/articles/d98edd0cfc2441)(Zenn)、[PostmanでgRPC APIのテスト](https://qiita.com/yokawasa/items/8a55dd427230de6374bc)(Qiita) — Postman での gRPC の扱い
- [IDOLY PRIDEにおけるゲームAPIの開発](https://technote.qualiarts.jp/article/31/)(QualiArts) — protoからAPIドキュメントを生成し、画面ごとに必要なAPIをクライアントとすり合わせる

### Atlas(7章)

- [chinpanGX/api-codegen](https://github.com/chinpanGX/api-codegen) — REST時代のコード生成ツール
- Atlasのコード: サーバーの認証は `Server/src/service/auth_service.rs`・`device_service.rs`・`extractor.rs`、差分は `Server/src/api/scout.rs`・`player.rs`(`build_full_player_diff`)。クライアントは `AccessTokenRefresher.cs`・`PlayerDiffApplier.cs`
- Atlasの `Shared/docs/design.md`「6.9 通信エラーダイアログ・Loading(設計案)」 — 二重送信で何が壊れるかをエンドポイントごとに調べた表
