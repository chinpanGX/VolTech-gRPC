---
title: "エラー"
status: confirmed
area: api
related: [api/auth, server/auth, server/status, client/error-handling, client/response-items, ops/testing]
proto: []
code: [server/rust/error, server/rust/error-build, server/rust/error/errors.toml, sandbox/errors.toml, tools/protoc-gen-voltech-grpc/src/Errors, Unity/com.chinpangx.voltech.grpc/Runtime/Client/Errors]
updated: 2026-10-11
---

# エラー

## 返し方

| 種類 | ステータス | `ErrorInfo` の domain / reason |
|---|---|---|
| 業務エラー(メンテナンス・強制アップデート等を含む) | `FAILED_PRECONDITION` | アプリの業務エラーはアプリの domain(例: `sandbox` / `INSUFFICIENT_GEMS`)、基盤の予約エラーは `voltech`(例: `voltech` / `MAINTENANCE`) |
| サーバー内部エラー | `INTERNAL` | `voltech` / `INTERNAL_ERROR`(詳細はログにだけ出す) |
| 認証エラー | `UNAUTHENTICATED` | なし([認証](auth.md)) |

- gRPCではエラー時にレスポンスメッセージを返せないので、**リッチエラーモデル**で運ぶ。`google.rpc.Status` の details に Google 標準の `google.rpc.ErrorInfo` を入れ、トレーラー `grpc-status-details-bin` で返す(Rust は tonic-types、C# は `Google.Api.CommonProtos` で扱える)

```proto
// google/rpc/error_details.proto(Googleが提供する定義)
message ErrorInfo {
  string reason = 1;                 // エラーの名前(例: INSUFFICIENT_GEMS)
  string domain = 2;                 // エラーの持ち主(基盤の予約エラーは voltech、アプリの業務エラーはアプリの domain)
  map<string, string> metadata = 3;  // action の上書き、メンテナンスの終了予定など
}
```

- **エラーは domain と reason の組で見分ける**(`ErrorCode`)。reason が同じでも domain が違えば別のエラー
- 基盤の予約エラーは domain `voltech` で、アプリは使えない。アプリが予約エラーを変えたり消したりできないので、基盤を取り込むたびに手で合わせる必要が無い

## エラーの定義ファイル(errors.toml)

エラーコードと既定の action は、domain ごとの定義ファイル `errors.toml` に書き、サーバー(Rust)とクライアント(C#)の定数を生成する(ここが唯一の定義元)。

| 定義ファイル | domain | Rust | C# |
|---|---|---|---|
| `server/rust/error/errors.toml`(基盤の予約エラー) | `voltech` | `voltech_grpc_error::reserved`(`voltech-grpc-error` の build.rs が生成) | `VolTech.Client.ReservedErrors`(パッケージに同梱。`make errors-gen`) |
| アプリの `errors.toml`(sandbox は `sandbox/errors.toml`) | アプリが決める(sandbox は `sandbox`) | アプリの build.rs で `voltech_grpc_error_build::compile` を呼ぶ(sandbox は `proto::errors`) | `protoc-gen-voltech-grpc errors` で生成する(sandbox は `Sandbox.V1.SandboxErrors`。`make sandbox-errors-gen`) |

```toml
# アプリの errors.toml(sandbox/errors.toml から抜粋)
domain = "sandbox"

[[errors]]
reason = "INSUFFICIENT_GEMS"
action = "stay"
description = "ジェムが足りず、ガチャを引けない。"
```

- **domain**: 小文字・数字・ハイフンを `.` でつないだ名前(例: `my-app`)。`voltech` はアプリでは使えない
- **reason**: 大文字のスネークケース、63文字以内(AIP-193 の `ErrorInfo.reason` の規則)。同じファイルの中で重複させない。生成するコードの定数とぶつかる `DOMAIN`・`ALL` は使えない
- **action**: 既定の action(`stay`・`retry`・`home`・`title`・`update`)
- **client_only**: クライアントだけが使うエラー(サーバーは返さない)。Rust の定数を作らない
- **description**: 用途。生成するコードのコメントになる。必ず書く
- 規則に合わないと、生成(build.rs・`protoc-gen-voltech-grpc errors`)がエラーになる
- **消した reason は使い回さない**(古いアプリが別の意味に受け取る)
- **文言キー**は domain と reason から決まりで作る(例: `sandbox` / `INSUFFICIENT_GEMS` → `error.sandbox.insufficient_gems`)

## 返す・受け取る

- サーバー: `voltech_grpc_error::failed_precondition(code, message)` で業務エラーを返す。`code` は生成した定数(例: `proto::errors::INSUFFICIENT_GEMS`)。表示用の値を付けるときは `failed_precondition_with_metadata`。内部エラーは `internal(err)`(詳細はログにだけ出る)
- クライアント: `ApiError.Code` が `ErrorCode`(domain と reason)になる。画面で自分で処理したいエラーは、生成した定数を `handledCodes` に渡す(例: `new[] { SandboxErrors.InsufficientGems }`)。予約エラーは渡しても、必ず共通エラーハンドラーが処理する([エラー処理](../client/error-handling.md))
- クライアントの既定の action は、`ErrorPolicyProvider` に生成した一覧(例: `SandboxErrors.All`)を渡して引く。予約エラー(`ReservedErrors.All`)は自動で入る
- **サーバーによる action の上書き**: 必要なときだけ `metadata["action"]` に action の名前を入れる。クライアントが読むのは `ERROR_ACTION_STAY`・`ERROR_ACTION_RETRY`・`ERROR_ACTION_HOME`・`ERROR_ACTION_TITLE`・`ERROR_ACTION_UPDATE` で、知らない値なら既定の action を使う。画面名は指定しない
- **知らないコード**(古いアプリに無い domain・reason)は `UNKNOWN_ERROR` と同じ挙動にする。サーバーが後からコードを足しても、古いアプリは壊れない
- **追加**は `errors.toml` に足して生成し直し、文言を足すだけ
- **文言への値の埋め込み**は v1 では対応しない。例外はメンテナンスの終了予定(下記)。必要になったら `metadata` で値を送る(キー `action` は避ける)

## action とダイアログ

| action | ボタン | 挙動 |
|---|---|---|
| Stay | OK | 閉じて画面に留まる |
| Retry | 再試行 / タイトルへ | 再試行: 同じRequestIDで再送 / タイトルへ: タイトルに戻る |
| Home | ホームへ | ホーム画面に戻る |
| Title | タイトルへ | タイトルに戻る(セッション情報を破棄) |
| Update | ストアへ | ストアのページを開く。アプリは先へ進めない |

## 予約エラー(domain: voltech)

一覧と既定の action は [`server/rust/error/errors.toml`](../../../server/rust/error/errors.toml) が正。ここには、どこが返すかと注意だけを書く。

| reason | 返す側 | 注意 |
|---|---|---|
| `NETWORK_ERROR` | クライアント | 通信断・タイムアウト |
| `INTERNAL_ERROR` | サーバー | 一時的な障害は再試行で直り得る |
| `UNKNOWN_ERROR` | クライアント | 想定外のステータス、知らないエラー |
| `MAINTENANCE` | サーバー([サーバーの状態](../server/status.md)) | 終了予定を `metadata` の `end_at` に入れる(下記) |
| `UPDATE_REQUIRED` | サーバー([サーバーの状態](../server/status.md)) | リリース済みのアプリには後から足せないので v1 から入れる |
| `ACCOUNT_SUSPENDED` | サーバー([認証(サーバー)](../server/auth.md)) | BAN・アカウント停止 |
| `DATE_CHANGED` | サーバー([サーバーの状態](../server/status.md)) | サインインし直し、所持データを全件取り直す |
| `CLIENT_ERROR` | クライアント | 共通レスポンス項目の反映処理での例外など([共通レスポンス項目の反映](../client/response-items.md))。通信エラーとは違う文言にする |

## メンテナンス中の表示

- `MAINTENANCE` を返すときは、終了予定時刻などの表示用の値を `metadata` に入れる。クライアントはそれをダイアログに表示する。キーは `end_at`(UTC の RFC 3339。[サーバーの状態](../server/status.md))。クライアントでは `ApiError.Metadata` で読める
- メンテナンスの判定は全リクエスト共通([サーバーの状態](../server/status.md))なので、タイトル画面で最初に呼ぶトークン発行でも同じ値が返る。メンテナンスを知らせる専用のAPIは作らない
- 運営からの「お知らせ」は、認証ありの普通のAPIで取得する(静的なファイルでは配信しない)。トークン発行は認証不要なので、タイトル画面でも先にトークンを取ってから呼べる
- メンテナンス中でも呼べるAPI(判定の対象外)は v1 では作らない。メンテナンス中はお知らせのAPIも `MAINTENANCE` になるが、`metadata` の値で必要な情報は表示できる

## 実装で決めたこと

- エラーコードは `.proto` の enum(`ErrorCode`)と、カスタムオプション `default_action` で定義していたのをやめ、domain ごとの `errors.toml` にした(2026-10-10)。enum だと、基盤の予約コードをアプリの `error_code.proto` に書き写す必要があり、アプリが予約コードの番号や action を変えたり消したりしても気づけなかった。`error_options.proto` をアプリへ受け渡す必要も無くなった
