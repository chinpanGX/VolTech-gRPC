---
title: "採らなかった案(索引)"
status: index
area: decisions
sections: ["8"]
source: "docs/feature-grpc-foundation.md"
updated: 2026-10-10
---

# 採らなかった案(索引)

> 各案の本文(理由を含む)は、それぞれのページ末尾の「採らなかった案」にある。ここは入口の一覧で、本文は持たない。

## [概要(目的・範囲・成果物・進め方・全体構成)](../overview.md)(3件)

- サンプルの置き場所: テンプレートのアプリの雛形に置き、アプリを作るときに書き換える
- サンプルの置き場所: テンプレートの中の `samples/` に置く
- サンプルの機能: Atlasのチャット機能

## [運用(パッケージとテンプレート)](../repo/operations.md)(7件)

- リポジトリの運用: GitHubのフォーク、「Use this template」
- パッケージにする範囲: 基盤をすべてパッケージにする
- クライアントの基盤: テンプレートに置き、Unity プロジェクトから `file:` で参照する
- クライアントの基盤: クライアントの基盤だけで1つのリポジトリにする
- パッケージのリポジトリ: Unity の通信基盤・サーバーのパッケージ・proto ツールで、リポジトリを分ける
- パッケージとテンプレート: 同じリポジトリで管理する
- パッケージにする範囲: パッケージにしない(すべて clone して取り込む)

## [リポジトリ名と命名](../repo/naming.md)(7件)

- リポジトリ名: 後ろに通信方式(gRPC)を付ける
- リポジトリ名: 技術の名前(Rust・Unity)を入れる
- テンプレート: サンプルとは別のテンプレートのリポジトリを作る
- 名前: パッケージとテンプレートで頭をそろえる(GameGrpcKit + GameGrpcServer)
- 名前: 開発名(UniGrpcKit など)のまま公開する
- Rust のクレート: 2つにまとめる
- `.proto` のパッケージ名: 機能ごとに分ける

## [サーバーの分け方](../repo/server-split.md)(1件)

- サーバーの分け方: マイクロサービスに分ける

## [フォルダ構成](../repo/folder-layout.md)(1件)

- テンプレートの構成: 基盤を `foundation/`、アプリを `app/` に分け、Unity プロジェクトを `app/client` に置く

## [基本規約(型の定義元・通信方式・nullable と oneof)](../api/conventions.md)(1件)

- 通信方式: v1からストリーミングを使う

## [コード生成](../api/codegen.md)(4件)

- コード生成: RoslynのSource Generator
- コード生成: Unityのエディタ拡張でリフレクションして生成
- コード生成: プラグインをRustで書く
- コード生成: プラグインを作るまで、実行時に Descriptor から読む(`RpcSettings` を `MethodDescriptor` から作る、`player_diff` をフィールド名で探す)

## [エラー](../api/errors.md)(4件)

- エラーの返し方: details に独自のメッセージを定義する
- エラー時の挙動: エラーマスター(code / 文言キー / action)で管理する(実務の方式)
- エラー時の挙動: サーバーが遷移先の画面を指示する
- エラー時の挙動: C#に「code → action」の表を手書きする、プラグインで表を生成する

## [認証](../api/auth.md)(2件)

- `player_id` の発行: サインアップで発行する(Atlasの方式)
- `player_id` の発行: デバイス登録でプレイヤーと初期データまで作る

## [冪等性(RequestID と idempotency_level)](../api/idempotency.md)(2件)

- 冪等性の指定: 独自のカスタムオプション、サービスを分けて表す
- 冪等性: RequestIDをリクエストメッセージのフィールドに持つ(AIP-155)

## [共通レスポンス項目](../api/common-response.md)(4件)

- playerDiffの載せ方: メタデータ(トレーラー)で返す
- playerDiffの載せ方: 全レスポンスを共通の入れ物(`{ PlayerDiff diff; Any body; }`)で包む
- playerDiffの値: 変化量(ジェム-100)で持つ
- バッジ: 種類をマスターで持つ

## [リクエストの処理の流れ](../server/request-flow.md)(2件)

- サーバーの共通処理: 冪等性・トランザクション・コレクターを tower Layer にする
- サーバーの共通処理: 認証を `tonic::Interceptor` で書く

## [with_transaction() と冪等性](../server/transaction.md)(1件)

- 冪等性の保存: 一定期間すべてのレスポンスを保存する(Stripe方式)

## [コレクター(差分・バッジ・通知)](../server/collectors.md)(3件)

- 差分の集め方: ハンドラーが手で組み立てる(Atlasの方式)
- 差分の集め方: 自動生成したデータアクセス層が変更を検知する(学マスの方式)
- バッジ: 「どのAPIでどの種類を返すか」の表を持つ(実務の方式)

## [認証(サーバー)](../server/auth.md)(2件)

- 認証の情報の渡し方: クライアントが `device_id`・`player_id` をメタデータで送る
- `player_id` の引き方: `access_tokens` にも `player_id` を持たせる

## [サーバーの状態(メンテナンス・強制アップデート・日付変更)](../server/status.md)(1件)

- サーバーの状態: メンテナンスなどの状態をDBに置く

## [構成](../client/structure.md)(1件)

- クライアントの本体: 本体のAPIを `ValueTask` にし、Unity用の層で UniTask に変える

## [ラッパー](../client/wrapper.md)(4件)

- ラッパーの形: APIごとに1クラス(実務のRESTの `ApiBase` の形)
- ラッパーの形: ラッパーを作らず、`ApiCore` を直接呼ぶ
- ラッパーの形: メソッド名を `SendAsync` にそろえてオーバーロードする
- ラッパー: 呼び出し側がキャンセルできる(`CancellationToken` を受け取る)

## [ApiCore の共通処理](../client/api-core.md)(2件)

- 冪等性: RequestIDを Interceptor で採番する
- クライアントの共通処理: Interceptor で書く

## [エラー処理](../client/error-handling.md)(6件)

- リトライ: 通信層はエラーを必ず呼び出し側に返す
- リトライ: 自動リトライ、Grpc.Net.Client の組み込みリトライ
- エラーの返し方: 呼び出し側に例外を投げる
- カスタムエラーハンドラー: switch式で振り分けを1か所に書く
- カスタムエラーハンドラー: 担当コードをプロパティで持ち、起動時にだけチェックする
- エラー表示: 基盤にトースターを持つ

## [認証とアカウント情報](../client/auth-account.md)(4件)

- アカウント情報の保存: JsonUtility で JSON にする
- アカウント情報の保存: BinaryWriter で手書き、MessagePack-CSharp
- アカウント情報の保存: `BinaryFormatter`
- アカウント情報の保存: Supplement にパッケージとして依存する

## [共通レスポンス項目の反映](../client/response-items.md)(4件)

- 共通レスポンス項目の反映: 全APIで汎用に使える成功後のフック(`IApiResponseHook`)
- 共通レスポンス項目の反映: アプリのServiceクラスがそれぞれ反映する(実務・Atlasの方式)
- Repository の操作: API のレスポンスの型で受け取る(`ReplaceAll(SignInResponse)`・`Apply(PlayerDiff)`)
- アイテムの削除: `removed` を無くし、個数 0 の Upsert で消す

## [接続設定](../client/connection.md)(1件)

- 接続設定: 開発サーバーで証明書の検証を飛ばす(`SkipCertificateVerification`)

## [サーバーが無いときの差し替えとテスト](../client/fake-testing.md)(1件)

- サーバーが無いときの差し替え: ラッパーの interface を生成してモックを作る
