---
title: "冪等性(RequestID と idempotency_level)"
status: confirmed
area: api
sections: ["3.6"]
source: "docs/feature-grpc-foundation.md"
related: [server/transaction]
proto: [sandbox/proto/sandbox/v1/sandbox.proto]
updated: 2026-10-06
---

# 冪等性(RequestID と idempotency_level)

学マス資料 p.51 のレスポンスキャッシュを参考にした仕組み。レスポンスを受け取る前に通信が切れて再送しても、処理は一度だけ行われる。

- クライアントは1回の操作につき1つRequestIDを採番し、メタデータ `x-request-id` に付けて送る(UUID、ハイフン無しの32文字)。「再試行」では同じRequestIDで再送する
- サーバーは同じRequestIDが再送されたら、処理し直さずに保存済みのレスポンスを返す(仕組みは[4.3](../server/transaction.md))
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
- `IDEMPOTENT` の宣言が正しいかは機械的に判定できないので、`.proto` の差分をレビューで確認し、そのAPIの結合テストで確かめる([6章](../ops/index.md))
- **サインイン**は `IDEMPOTENT` にする。全件データを返すので、対象にすると保存量とサインインが集中する時間帯の書き込みが重くなる。読み取りが中心なので、再実行されても結果は変わらない
  - 付与などの状態変更(ログインボーナス等)は別のAPIにして、この仕組みに乗せる(再送されても二重に付与されず、保存済みのレスポンスで演出も出せる)。サインイン中の書き込みは、最終ログイン日時の更新のように何回実行しても同じものに限る
- **認証前のAPI**(デバイス登録など)は、どのプレイヤーか決まらないので、APIごとに冪等に作って `IDEMPOTENT` と宣言する
- **サインアップ**(プレイヤーの作成)は、プレイヤーの行がまだ無いのでこの仕組みを使えない。`IDEMPOTENT` とし、**作成済みならエラーにせず作成済みのプレイヤーを返す**

## 採らなかった案

| 項目 | 採らなかった案 | 理由 |
|---|---|---|
| 冪等性の指定 | 独自のカスタムオプション、サービスを分けて表す | 標準の `idempotency_level` で表せる |
| 冪等性 | RequestIDをリクエストメッセージのフィールドに持つ(AIP-155) | 全APIに一律に適用するので、メタデータに載せる |
