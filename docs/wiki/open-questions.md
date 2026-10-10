---
title: "未確定・保留"
status: open
area: root
updated: 2026-10-11
---

# 未確定・保留

決まった項目は行を消し、決めた内容を該当ページに書く。番号は他のページから参照されるので、振り直さない。

| # | 項目 | 状態 | 節 |
|---|---|---|---|
| 1 | クレートの細かい分け方(ドメインのクレートが生成コードに直接依存するか) | 実装時に決める | [フォルダ構成](repo/folder-layout.md), [レイヤー構成](server/layers.md) |
| 4 | 細かい名前(パッケージの中のフォルダ名) | 実装時に決める。ヘッダー名と `MAINTENANCE` の `metadata` のキー名は決めた([メタデータ](api/metadata.md)、[サーバーの状態](server/status.md)) | [ラッパー](client/wrapper.md) |
| 5 | サインインの全件の置き換えと、`IPlayerDiffApplier` による自動の反映の区別(フィールド名を分けるなど) | 実装時に決める | [共通レスポンス項目](api/common-response.md), [共通レスポンス項目の反映](client/response-items.md) |
| 6 | サーバーでも `skip_auth` を読むか(`prost-reflect`) | v1の後に判断する | [認証(サーバー)](server/auth.md) |
| 7 | ソフトメンテ(IPの許可リスト)と、設定の置き場所をAWSの設定サービスにすること | 一般のユーザーに公開する前に足す | [サーバーの状態](server/status.md) |
| 8 | `player_id` の許可リスト、通常メンテと緊急メンテの出し分け、管理画面 | 運用が大きくなってから | [サーバーの状態](server/status.md) |
| 13 | AtlasのREST版を残すブランチの名前・作る時期 | 乗せ換えを始めるときに決める | [概要](overview.md) |
| 14 | コンテナ化とAWSへのデプロイの進め方 | gRPCへの移行とAPIの実装が終わった後 | — |
| 16 | パッケージのリリースの手順(タグの付け方、変更履歴) | パッケージを作るときに決める | [運用](repo/operations.md) |
| 17 | 基盤を `voltech-grpc` に移す時期と、Unity の `Api` のアセンブリにある基盤の部分(認証・Unity 用の層)の分け方 | **解決(2026-10-10)**。VolTech-gRPC に分けた。認証は `VolTech.Client.Auth`、Unity 用の層は `VolTech.Client.Unity` のアセンブリにした([当面の実装方針](repo/current-plan.md)) | [運用](repo/operations.md)〜[フォルダ構成](repo/folder-layout.md), [当面の実装方針](repo/current-plan.md) |
| 19 | 保存したアカウント情報が使えないときのエラーの細部(クライアントの専用エラーの形、サーバーが無効にした理由のエラーコード名、引き継ぎ・削除で行に残す項目) | 方針は決めた([認証](api/auth.md)、[認証とアカウント情報](client/auth-account.md))。引き継ぎ・削除の機能と、保存の仕組み(`EncryptedFileAccountStore`)を作るときに決める | [認証](api/auth.md) |
| 20 | デバイス登録の回数の制限(認証なしで呼べ、毎回 Argon2 の計算と `devices` の行の追加をするので、連打でサーバーの資源を使い切られる)。サーバーで制限するか、インフラ(WAF など)で制限するか | 一般のユーザーに公開する前に決める | [認証(サーバー)](server/auth.md) |
| 21 | DLC(アセット・マスターデータ)のバージョンを送り、古ければ取り直させるか | DLC の仕組みを入れるときに決める。古いアプリは送らないので、「ヘッダーが無ければ判定しない」決まりのまま後から足せる | [メタデータ](api/metadata.md), [サーバーの状態](server/status.md) |
| 22 | ストアの区別(同じプラットフォームで複数のストアに出すときの、ストアごとの最低バージョン)。課金のレシート検証のストアは、購入 API のリクエストの項目にする | 課金や2つ目のストアを入れるときに決める。`x-app-platform` の値を細かくするか、`x-app-store` を足す | [メタデータ](api/metadata.md), [サーバーの状態](server/status.md) |
| 24 | カスタムエラーハンドラーの属性で、担当するエラーコードをどう書くか。`ErrorCode` が enum から構造体(domain と reason)になったので、属性の引数に定数として書けない(文字列で domain と reason を書く、生成コードに定数の文字列を足す、など) | カスタムエラーハンドラーを作るときに決める | [エラー処理](client/error-handling.md), [エラー](api/errors.md) |
