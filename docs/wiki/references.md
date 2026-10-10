---
title: "参考資料"
status: reference
area: root
sections: ["10"]
source: "docs/feature-grpc-foundation.md"
updated: 2026-10-06
---

# 参考資料

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
