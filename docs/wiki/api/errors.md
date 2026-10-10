---
title: "エラー"
status: confirmed
area: api
sections: ["3.5"]
source: "docs/feature-grpc-foundation.md"
related: [api/auth, server/auth, server/status, client/error-handling, client/response-items, ops/testing]
proto: [proto/app/v1/error_code.proto]
code: [Server/framework/error]
updated: 2026-10-10
---

# エラー

## 返し方

| 種類 | ステータス | `ErrorInfo.reason` |
|---|---|---|
| 業務エラー(メンテナンス・強制アップデート等を含む) | `FAILED_PRECONDITION` | 業務ごとのコード(例: `ERROR_CODE_INSUFFICIENT_GEMS`) |
| サーバー内部エラー | `INTERNAL` | `ERROR_CODE_INTERNAL_ERROR`(詳細はログにだけ出す) |
| 認証エラー | `UNAUTHENTICATED` | なし([3.7](auth.md)) |

- gRPCではエラー時にレスポンスメッセージを返せないので、**リッチエラーモデル**で運ぶ。`google.rpc.Status` の details に Google 標準の `google.rpc.ErrorInfo` を入れ、トレーラー `grpc-status-details-bin` で返す(Rust は tonic-types、C# は `Google.Api.CommonProtos` で扱える)

```proto
// google/rpc/error_details.proto(Googleが提供する定義)
message ErrorInfo {
  string reason = 1;                 // ErrorCode の値の名前
  string domain = 2;                 // 固定値。クライアントは判定に使わない
  map<string, string> metadata = 3;  // action の上書き、メンテナンスの終了予定など
}
```

## エラーコードと action の定義

エラーコードと action は `.proto` の enum で定義し、サーバー・クライアントの両方に生成する(ここが唯一の定義元)。

```proto
// voltech/v1/error_options.proto(基盤。Server/framework/error/proto に置く)
package voltech.v1;

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

アプリのエラーコードは [`app/v1/error_code.proto`](../../../proto/app/v1/error_code.proto) の `ErrorCode` に定義する(一覧はそちらを見る。ここには書き写さない)。

- 番号: 1〜999 は基盤の予約コード(`NETWORK_ERROR`・`INTERNAL_ERROR`・`MAINTENANCE`・`ACCOUNT_SUSPENDED` など)、1000 以降をアプリが足す。業務エラーは機能ごとに番号の範囲を分け、範囲を `.proto` のコメントに書く
- 消した番号と名前は `reserved` にする
- 書き方の例:

```proto
enum ErrorCode {
  ERROR_CODE_UNSPECIFIED = 0;
  // 1〜999: 基盤の予約コード
  ERROR_CODE_INTERNAL_ERROR = 2 [(voltech.v1.default_action) = ERROR_ACTION_RETRY];
  // 1000〜1099: ガチャ
  ERROR_CODE_INSUFFICIENT_GEMS = 1001 [(voltech.v1.default_action) = ERROR_ACTION_STAY];
}
```

- **`reason`** には enum の値の名前を入れる(サーバーは `as_str_name()`、クライアントは `ErrorCode.Descriptor.FindValueByName()` で相互に変換する)。名前は AIP-193 の規則(大文字のスネークケース、63文字以内)を満たす
- **既定の action** は、enum の各値にカスタムオプション `default_action` で付ける。付いていない値は Title として扱う
- **文言キー**は enum の名前から決まりで作る(例: `ERROR_CODE_INSUFFICIENT_GEMS` → `error.insufficient_gems`)
- **サーバーによる action の上書き**: 必要なときだけ `metadata["action"]` に `ErrorAction` の名前を入れる。指定できるのは5種類の action だけで、画面名は指定しない
- **知らないコード**(古いアプリに無い値)は `UNKNOWN_ERROR` と同じ挙動にする。サーバーが後からコードを足しても、古いアプリは壊れない
- **追加**は `.proto` に1行足して生成し直し、文言を足すだけ。**削除**した番号と名前は `reserved` にする([6章](../ops/index.md))
- **文言への値の埋め込み**は v1 では対応しない。例外はメンテナンスの終了予定(下記)。必要になったら `metadata` で値を送る(キー `action` は避ける)

## action とダイアログ

| action | ボタン | 挙動 |
|---|---|---|
| Stay | OK | 閉じて画面に留まる |
| Retry | 再試行 / タイトルへ | 再試行: 同じRequestIDで再送 / タイトルへ: タイトルに戻る |
| Home | ホームへ | ホーム画面に戻る |
| Title | タイトルへ | タイトルに戻る(セッション情報を破棄) |
| Update | ストアへ | ストアのページを開く。アプリは先へ進めない |

## 予約コード

| エラーコード(`ERROR_CODE_` を省略) | 返す側 | action | 用途 |
|---|---|---|---|
| `NETWORK_ERROR` | クライアント | Retry | 通信断・タイムアウト |
| `INTERNAL_ERROR` | サーバー | Retry | サーバー内部エラー。一時的な障害は再試行で直り得る |
| `UNKNOWN_ERROR` | クライアント | Title | 想定外のステータス、知らないコード |
| `MAINTENANCE` | サーバー | Title | メンテナンス中([4.6](../server/status.md)) |
| `UPDATE_REQUIRED` | サーバー | Update | 強制アップデート([4.6](../server/status.md))。リリース済みのアプリには後から足せないので v1 から入れる |
| `ACCOUNT_SUSPENDED` | サーバー | Title | BAN・アカウント停止([4.5](../server/auth.md)) |
| `DATE_CHANGED` | サーバー | Title | 日付変更([4.6](../server/status.md))。サインインし直し、所持データを全件取り直す |
| `CLIENT_ERROR` | クライアント | Title | クライアントのバグ(共通レスポンス項目の反映処理での例外など。[5.6](../client/response-items.md))。通信エラーとは違う文言にする |

- パッケージ([4.5](../server/auth.md)、[4.6](../server/status.md))が返す予約コード(`MAINTENANCE`・`UPDATE_REQUIRED`・`DATE_CHANGED`・`ACCOUNT_SUSPENDED`)は、基盤の `voltech-grpc-error` クレートに名前の文字列の定数としてまとめて持つ(`voltech_grpc_error::code`。`ErrorCode` の enum はアプリ側で生成されるため)。テンプレートの `error_code.proto` の予約コードと一致しているかを、テストで確かめる([6.3](../ops/testing.md))

## メンテナンス中の表示

- `MAINTENANCE` を返すときは、終了予定時刻などの表示用の値を `metadata` に入れる。クライアントはそれをダイアログに表示する。キーは `end_at`(UTC の RFC 3339。[4.6](../server/status.md))。クライアントでは `ApiError.Metadata` で読める
- メンテナンスの判定は全リクエスト共通([4.6](../server/status.md))なので、タイトル画面で最初に呼ぶトークン発行でも同じ値が返る。メンテナンスを知らせる専用のAPIは作らない
- 運営からの「お知らせ」は、認証ありの普通のAPIで取得する(静的なファイルでは配信しない)。トークン発行は認証不要なので、タイトル画面でも先にトークンを取ってから呼べる
- メンテナンス中でも呼べるAPI(判定の対象外)は v1 では作らない。メンテナンス中はお知らせのAPIも `MAINTENANCE` になるが、`metadata` の値で必要な情報は表示できる

## 採らなかった案

| 項目 | 採らなかった案 | 理由 |
|---|---|---|
| エラーの返し方 | details に独自のメッセージを定義する | 標準の `ErrorInfo` なら Rust・C# ともライブラリの型がそのまま使え、引数も `metadata` で運べる |
| エラー時の挙動 | エラーマスター(code / 文言キー / action)で管理する(実務の方式) | テンプレートが master-data-pipeline に依存し、マスターを読む前のエラーを扱えない。将来差し替えられる形にはしておく([5.4](../client/error-handling.md)) |
| エラー時の挙動 | サーバーが遷移先の画面を指示する | サーバーがクライアントの画面構成を知ることになる。action の上書きだけを残した |
| エラー時の挙動 | C#に「code → action」の表を手書きする、プラグインで表を生成する | コードを足すたびに表も足す必要がある / プラグインの機能が増える |
