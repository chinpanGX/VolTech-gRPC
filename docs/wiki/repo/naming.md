---
title: "リポジトリ名と命名"
status: current
area: repo
sections: ["2.2"]
source: "docs/feature-grpc-foundation.md"
related: [repo/current-plan, repo/folder-layout, ops/local-override]
proto: []
updated: 2026-10-10
---

# リポジトリ名と命名

基盤(パッケージ)の名前は **VolTech gRPC** にする。基盤はこのリポジトリ(`VolTech-gRPC`)に分け、`grpc-foundation-samples` は、基盤を使うサンプル兼テンプレートとして残す(分け方は [当面の実装方針](current-plan.md))。

| もの | 名前 |
|---|---|
| 基盤(パッケージ)のリポジトリ | **`VolTech-gRPC`**(このリポジトリ。計画では `voltech-grpc` と書いていた) |
| GitHub の説明文 | gRPC foundation for Unity × Rust game backends |
| GitHub のトピック | `grpc` `unity` `rust` `game-backend` `mobile-game` |
| サーバーのフォルダ(基盤のリポジトリの中) | `server/rust/`(Go 版を足したら `server/go/`) |
| Rust のクレート | `voltech-grpc-auth`・`voltech-grpc-error`・`voltech-grpc-status`・`voltech-grpc-transaction`(機能ごと) |
| UPM パッケージ | `com.chinpangx.voltech.grpc`(表示名: VolTech gRPC)。`chinpangx` は `name` だけに使い、表示名・asmdef・名前空間には入れない |
| C# の名前空間(クライアントの基盤) | `VolTech.Client`。認証は `VolTech.Client.Auth`、Unity 用の層は `VolTech.Client.Unity`、開発用のツールは `VolTech.Client.Tracing`・`VolTech.Client.Editor`。asmdef の名前も同じ |
| C# の名前空間(基盤の `.proto` から生成したコード) | `VolTech.V1`(`option csharp_namespace` で指定する) |
| protoc プラグイン | `protoc-gen-voltech-grpc` |
| `.proto` のパッケージ名(基盤の `device.proto` など) | `voltech.v1`(技術の名前は入れない) |
| サンプル兼テンプレートのリポジトリ | `grpc-foundation-samples` |
| 基盤の動作確認用のプロジェクト | `sandbox/`(このリポジトリの中。配布しない。`.proto` のパッケージは `sandbox.v1`) |

- **リポジトリ名に、サーバーの言語(Rust)や Unity を入れない**。Go 版のサーバーを足す予定があり、言語を入れると後で名前が合わなくなる。使っている技術は、GitHub の説明文とトピックで伝える
- サーバーの言語を増やすときは、リポジトリを分けずに、基盤のリポジトリの中に `server/<言語>/` のフォルダを足す
- 基盤は、サンプル兼テンプレートとは別のリポジトリにする
  - テンプレートの中に置かないのは、clone したアプリに基盤のソースのコピーが付いてきて、参照しているものと手元のコピーがずれ得るため。更新のやり方(タグを上げる / upstream から merge する)も混ざる
  - 開発中に両方を同時に直すときは、手元のパスで参照を差し替える([6.4](../ops/local-override.md))
- **Rust のクレートは、機能ごとに分けたまま、頭に `voltech-grpc-` を付ける**。アプリは使う機能のクレートだけを依存に入れる。DB に残る名前(マイグレーションの履歴のテーブル名 `_auth_migrations` など)は、クレート名を変えても変えない
- `.proto` のパッケージ名は、基盤のものをすべて `voltech.v1` にまとめ、ファイル名で機能を分ける(`voltech/v1/device.proto`・`auth_options.proto`・`error_options.proto`)。アプリの `.proto` は `app.v1` のように、アプリの名前空間に置く
  - `.proto` のパッケージ名は gRPC のメソッドのパス(`/voltech.v1.DeviceService/Register` など)に入る。言語の違うサーバーでも同じクライアントでつながるように、全言語で同じにする。リリース後に変えると古いアプリと通信できなくなるので、最初のリリースの前に決めた
  - `.proto` のパッケージ名は、通信のパスやログに毎回出るので短くする。コードの名前は `option csharp_namespace = "VolTech.V1";` のように指定して、長いままにする
- クライアントの基盤はパッケージなので、サーバーの言語を増やしても重複しない

## 実装で決めたこと

- エディタの通信の Viewer のメニューは `Window > VolTech gRPC > Request Viewer` にする
- samples の `Api` のアセンブリに混ざっていた基盤の部分は、パッケージにするときに分けた。認証(`DeviceTokenProvider` など)は `VolTech.Client.Auth`、Unity 用の層(`ApiChannel` など)は `VolTech.Client.Unity`、基盤の `.proto` の生成コードは `VolTech.V1`、エラーコードの表(`ProtoErrorPolicyProvider`)は `VolTech.Client`(アプリの `ErrorCode` の `EnumDescriptor` を受け取る形にした)
- パッケージのテストのアセンブリは `VolTech.Client.Tests.EditMode` にする(`Tests.EditMode` のままだと、利用側の同じ名前の asmdef とぶつかる)

## 採らなかった案

| 項目 | 採らなかった案 | 理由 |
|---|---|---|
| リポジトリ名 | 後ろに通信方式(gRPC)を付ける | 増えるのはサーバーの言語。gRPC は設計全体の前提 |
| リポジトリ名 | `voltech-grpc-rust`・`voltech-grpc-unity` のように技術の名前を入れる | Go 版のサーバーを足したときに名前が合わなくなる。技術は説明文とトピックで伝える |
| テンプレート | サーバーのテンプレートのリポジトリ(`gamegrpcserver-rust`)を、サンプルとは別に作る。サーバーの言語ごとにテンプレートのリポジトリを分ける | サンプルが、そのままテンプレートとして使える形になっている。2つを同じ内容で保つ手間がかかる |
| 名前 | パッケージとテンプレートで頭をそろえる(GameGrpcKit + GameGrpcServer) | テンプレートはサンプルが兼ねることにしたので、別の名前は要らない |
| 名前 | 開発名のまま公開する(UniGrpcKit・GameGrpcServer・`unigr.v1`) | 公開するときの正式な名前を VolTech gRPC に決めたので、置き換えた |
| Rust のクレート | `voltech-grpc-server`・`voltech-grpc-core` の2つにまとめる | すでに機能ごと(認証・エラー・サーバーの状態・トランザクション)に分かれていて、アプリが要るものだけを入れられる。まとめるのは名前の変更ではなく構成の変更になる |
| `.proto` のパッケージ名 | 機能ごとに分ける(`voltech.auth.v1`・`voltech.error.v1`) | 基盤の `.proto` は数が少なく、1つにまとめてもぶつからない。RPC のパスも短くなる |
