---
title: "未確定・保留"
status: open
area: root
sections: ["9"]
source: "docs/feature-grpc-foundation.md"
updated: 2026-10-10
---

# 未確定・保留

決まった項目は行を消し、決めた内容を該当ページに書く。番号は他のページから参照されるので、振り直さない。

| # | 項目 | 状態 | 節 |
|---|---|---|---|
| 1 | クレートの細かい分け方(テンプレートの基盤のクレートの分け方、ドメインのクレートが生成コードに直接依存するか) | 実装時に決める | [2.4](repo/folder-layout.md), [4.1](server/layers.md) |
| 2 | テンプレートの `Client/`・`Server/` の雛形の中身 | サーバー側は、起動・デバイス登録・トークンの発行まで実装済み([2.6](repo/current-plan.md))。Unity 側は作るときに決める | [2.4](repo/folder-layout.md) |
| 3 | サンプル(テンプレート)の buf から、基盤の `.proto`(カスタムオプションの定義)を参照する方法(BSR に置く、コピーする、など) | 当面は手元のパスで参照する([2.6](repo/current-plan.md))。基盤を `voltech-grpc` に分けるとき(#17)に決め直す | [2.4](repo/folder-layout.md), [3.4](api/codegen.md) |
| 4 | 細かい名前(パッケージの中のフォルダ名) | 実装時に決める。ヘッダー名と `MAINTENANCE` の `metadata` のキー名は決めた([3.8](api/metadata.md)、[4.6](server/status.md)) | [5.2](client/wrapper.md) |
| 5 | サインインの全件の置き換えと、`IPlayerDiffApplier` による自動の反映の区別(フィールド名を分けるなど) | 実装時に決める | [3.9](api/common-response.md), [5.6](client/response-items.md) |
| 6 | サーバーでも `skip_auth` を読むか(`prost-reflect`) | v1の後に判断する | [4.5](server/auth.md) |
| 7 | ソフトメンテ(IPの許可リスト)と、設定の置き場所をAWSの設定サービスにすること | 一般のユーザーに公開する前に足す | [4.6](server/status.md) |
| 8 | `player_id` の許可リスト、通常メンテと緊急メンテの出し分け、管理画面 | 運用が大きくなってから | [4.6](server/status.md) |
| 13 | AtlasのREST版を残すブランチの名前・作る時期 | 乗せ換えを始めるときに決める | [1.4](overview.md) |
| 14 | コンテナ化とAWSへのデプロイの進め方 | gRPCへの移行とAPIの実装が終わった後 | — |
| 16 | パッケージのリリースの手順(タグの付け方、変更履歴) | パッケージを作るときに決める | [2.1](repo/operations.md) |
| 17 | 基盤を `voltech-grpc` に移す時期と、Unity の `Api` のアセンブリにある基盤の部分(認証・Unity 用の層)の分け方 | 移し方は決めた([2.6](repo/current-plan.md))。時期は、実装の形が固まってから決める | [2.1](repo/operations.md)〜[2.4](repo/folder-layout.md), [2.6](repo/current-plan.md) |
| 19 | 保存したアカウント情報が使えないときのエラーの細部(クライアントの専用エラーの形、サーバーが無効にした理由のエラーコード名、引き継ぎ・削除で行に残す項目) | 方針は決めた([認証](api/auth.md)、[認証とアカウント情報](client/auth-account.md))。引き継ぎ・削除の機能と、保存の仕組み(`EncryptedFileAccountStore`)を作るときに決める | [認証](api/auth.md) |
| 20 | デバイス登録の回数の制限(認証なしで呼べ、毎回 Argon2 の計算と `devices` の行の追加をするので、連打でサーバーの資源を使い切られる)。サーバーで制限するか、インフラ(WAF など)で制限するか | 一般のユーザーに公開する前に決める | [認証(サーバー)](server/auth.md) |
| 21 | DLC(アセット・マスターデータ)のバージョンを送り、古ければ取り直させるか | DLC の仕組みを入れるときに決める。古いアプリは送らないので、「ヘッダーが無ければ判定しない」決まりのまま後から足せる | [3.8](api/metadata.md), [4.6](server/status.md) |
| 22 | ストアの区別(同じプラットフォームで複数のストアに出すときの、ストアごとの最低バージョン)。課金のレシート検証のストアは、購入 API のリクエストの項目にする | 課金や2つ目のストアを入れるときに決める。`x-app-platform` の値を細かくするか、`x-app-store` を足す | [3.8](api/metadata.md), [4.6](server/status.md) |
