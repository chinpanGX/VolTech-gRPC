---
title: "接続設定"
status: confirmed
area: client
sections: ["5.7"]
source: "docs/feature-grpc-foundation.md"
related: [server/request-flow]
proto: []
code: [Unity/com.chinpangx.voltech.grpc/Runtime/Client/ApiCoreOptions.cs, Unity/com.chinpangx.voltech.grpc/Runtime/Unity/ApiChannel.cs]
updated: 2026-10-06
---

# 接続設定

| 項目 | 既定値 | 補足 |
|---|---|---|
| deadline(1回の通信全体の制限時間) | 15秒 | 全RPC共通。アプリ側から上書きできる。v1ではRPCごとの上書きはしない |
| `ConnectTimeout`(つながるまでの制限時間) | 10秒 | YetAnotherHttpHandler は既定では無効。アプリ側から上書きできる |
| 受信サイズの上限(`MaxReceiveMessageSize`) | 4MB(Grpc.Net.Client の既定) | 変更できるようにする。開発中は、レスポンスが上限の半分を超えたら警告ログを出す |
| HTTP/2 の keepalive | 無効(既定のまま) | 問題が出たら見直す |

- deadline は接続の部分も含むので `ConnectTimeout` は必須ではないが、つながらないときに早めに諦め、裏で接続を試み続けるのを止めるために設定する
- どちらのタイムアウトも `NETWORK_ERROR` になり、再試行のダイアログが出る。再試行は同じRequestIDなので安全。サーバーも同じ deadline で処理を打ち切ってロールバックする([4.2](../server/request-flow.md))
- 受信サイズの上限を超えると `UNKNOWN_ERROR` になり、サインインの全件データが大きくなると誰もサインインできなくなる。警告で早めに気づけるようにする
- **接続先**: ローカルは `http://` で、YetAnotherHttpHandler の `Http2Only = true` を付ける(h2c。指定しないとHTTP/1になる)。開発・本番のサーバーは `https://` で、公的な証明書(AWSならACM)を使う。接続先のURLはアプリ側から `ApiCore` に渡し、環境の切り替え方はアプリ側に任せる
- **実機からローカルのサーバーにつなぐとき**: WindowsのPCで立てたサーバーに、LAN内のIPアドレスで接続する(ファイアウォールでポートを開ける)。iOS・Androidの暗号化なしの通信の制限が効くかは、初めて実機で試すときに確かめる
- サーバーの前段のロードバランサーは、HTTP/2 を通せる必要がある(AWSのALBは対応、App Runnerは非対応)

## 採らなかった案

| 項目 | 採らなかった案 | 理由 |
|---|---|---|
| 接続設定 | 開発サーバーで証明書の検証を飛ばす(`SkipCertificateVerification`) | 開発サーバーも公的な証明書を使う |
