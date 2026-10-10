---
title: "コレクター(差分・バッジ・通知)"
status: confirmed
area: server
sections: ["4.4"]
source: "docs/feature-grpc-foundation.md"
related: [api/common-response]
proto: []
code: []
updated: 2026-10-10
---

# コレクター(差分・バッジ・通知)

共通レスポンス項目([3.9](../api/common-response.md))は、リクエスト単位のコレクターに集め、`with_transaction()` の最後にレスポンスへ詰める。ハンドラーは手で並べない。トランザクションがロールバックされたら中身も捨てる。

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
- バッジ・通知のコレクターも、アプリの `player` クレートに置く(`player::badge::BadgeCollector`・`player::notification::NotificationCollector`)。3つのコレクターは `player::collector::Collectors` にまとめ、`service/` の関数に1つの引数で渡す
- バッジの種類は `player::badge::BadgeKind`(`.proto` の `BadgeType` に対応)。判定はハンドラーが詰めるときに `api::badge::to_proto` で行い、記録された種類だけを今のデータから計算する。種類を足したら、ここに判定を足す(trait `BadgeResolver` にするのは、詰める処理を基盤に移すとき)
- サインインは `mark_all` で全種類のバッジを返す
- 日付で決まるバッジ(無料ガチャ)は、リクエストを受けたときのゲーム内の日付(`voltech_grpc_status::GameDate`。[サーバーの状態](status.md))で判定する

## 採らなかった案

| 項目 | 採らなかった案 | 理由 |
|---|---|---|
| 差分の集め方 | ハンドラーが手で組み立てる(Atlasの方式) | 副作用で変わったものを入れ忘れ得る |
| 差分の集め方 | 自動生成したデータアクセス層が変更を検知する(学マスの方式) | データアクセス層まで生成する大がかりなコード生成が要る |
| バッジ | 「どのAPIでどの種類を返すか」の表を持つ(実務の方式) | 書き込み関数が記録する形なら、表を手で保守しなくてよい |
