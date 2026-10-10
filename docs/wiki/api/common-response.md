---
title: "共通レスポンス項目"
status: confirmed
area: api
sections: ["3.9"]
source: "docs/feature-grpc-foundation.md"
related: [api/errors, server/transaction]
proto: [sandbox/proto/sandbox/v1/sandbox.proto]
updated: 2026-10-07
---

# 共通レスポンス項目

APIのレスポンスに共通して載せる項目。決まった名前のフィールドで持たせ、サーバーは集めて詰め、クライアントは自動で反映する。

| | `player_diff` | `badges` | `notifications` |
|---|---|---|---|
| 何か | DBのデータの変更。クライアントのキャッシュをサーバーに合わせる | DBのデータから計算した、表示用の状態 | リクエスト中に起きた出来事(報酬を得た、ミッションをクリアした) |
| DBに保存するか | する(業務データそのもの) | しない(その場で計算する) | しない |
| クライアントでの扱い | Repositoryを更新する | バッジの状態を更新する | 待ち行列に積み、画面が都合のよいときに出す |

```proto
// アプリの .proto(中身はアプリごとに違う。下は形の例で、サンプルの実際の定義は
// app/v1/player.proto・gacha.proto・badge.proto・notification.proto)
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

- 3つとも、エラーのレスポンスには付かない。レスポンスごと保存される([4.3](../server/transaction.md))ので、同じRequestIDで再送されても同じ内容が返り、通知が2回出ることはない
- **player_diff**
  - 値は変化量(ジェム-100)ではなく、**変更後の値**(ジェム残り200)で持つ。同じ差分を2回反映しても結果が変わらない
  - サインインは全件のスナップショットを返し、クライアントは差分としてではなく「置き換え」として反映する
- **badges**
  - 含まれない種類は「変化なし」
  - 種類は `.proto` の enum で持つ(種類を足すときはサーバーに判定処理を書くので、マスターで持つ利点が無い)。古いアプリは知らない種類を無視する
  - クライアントが同期しているデータから判定できるバッジ(例: 未受け取りのプレゼントの件数)は、サーバーから返さない。サーバーから返すのは、クライアントがデータを持っていないもの(お知らせ、ショップなど)だけ
- **notifications**
  - 出来事の種類ごとに中身が違うので `oneof` で表す。古いアプリは知らない種類を無視する
  - 「通知」は、運営からの「お知らせ」([3.5](errors.md))とは別物として呼び分ける
- 共通の項目を後から足すときも、同じ形(規約のフィールド名、サーバーのコレクター、クライアントの反映処理、使っていれば必須)で増やす

## 実装で決めたこと(サンプル)

- バッジは「無料ガチャ」(`BADGE_TYPE_FREE_GACHA`)。今日の無料ガチャをまだ引いていなければ表示。最後に引いた日はサーバーだけが持つので、サーバーが返すバッジの条件に合う
- 通知は「レアのアイテムを手に入れた」(`RareItemObtained`)
- サインイン(`SignInResponse`)は `badges` だけを持つ。全件を返すレスポンスなので、出来事(`notifications`)は無い
- 3つとも、使わないレスポンスには足さない(足すと、そのレスポンスの反映処理が必須になる)

## 採らなかった案

| 項目 | 採らなかった案 | 理由 |
|---|---|---|
| playerDiffの載せ方 | メタデータ(トレーラー)で返す | サイズ上限(8〜16KB程度)があり、サインインの全件が載らない(アプリボットは実際に上限を超えた) |
| playerDiffの載せ方 | 全レスポンスを共通の入れ物(`{ PlayerDiff diff; Any body; }`)で包む | APIごとの型が消える |
| playerDiffの値 | 変化量(ジェム-100)で持つ | 2回反映すると結果が変わり、保存済みのレスポンスの再送と両立しない |
| バッジ | 種類をマスターで持つ | 種類を足すときはサーバーに判定処理を書くので、マスターの利点が無い |
