---
title: "リポジトリ名と命名"
status: current
area: repo
related: [repo/current-plan, repo/folder-layout, guide/upgrade]
proto: []
updated: 2026-10-11
---

# リポジトリ名と命名

基盤(パッケージ)の名前は **VolTech gRPC** にする。基盤はこのリポジトリ(`VolTech-gRPC`)に分け、アプリはパッケージとして参照する。ガチャのサンプルは、このリポジトリの sandbox に置く([運用](operations.md)。分け方は [当面の実装方針](current-plan.md))。

| もの | 名前 |
|---|---|
| 基盤のリポジトリ | **`VolTech-gRPC`**(このリポジトリ。計画では `voltech-grpc` と書いていた) |
| GitHub の説明文 | gRPC foundation for Unity × Rust game backends |
| GitHub のトピック | `grpc` `unity` `rust` `game-backend` `mobile-game` |
| サーバーのフォルダ(基盤のリポジトリの中) | `server/rust/`(Go 版を足したら `server/go/`) |
| Rust のクレート | `voltech-grpc-auth`・`voltech-grpc-error`・`voltech-grpc-status`・`voltech-grpc-transaction`(機能ごと)。エラーの定数を生成する `voltech-grpc-error-build`(build.rs から使う) |
| UPM パッケージ | `com.chinpangx.voltech.grpc`(表示名: VolTech gRPC)。`chinpangx` は `name` だけに使い、表示名・asmdef・名前空間には入れない |
| C# の名前空間(クライアントの基盤) | `VolTech.Client`。認証は `VolTech.Client.Auth`、Unity 用の層は `VolTech.Client.Unity`、開発用のツールは `VolTech.Client.Tracing`・`VolTech.Client.Editor`。asmdef の名前も同じ |
| C# の名前空間(基盤の `.proto` から生成したコード) | `VolTech.V1`(`option csharp_namespace` で指定する) |
| protoc プラグイン | `protoc-gen-voltech-grpc` |
| `.proto` のパッケージ名(基盤の `device.proto` など) | `voltech.v1`(技術の名前は入れない) |
| 基盤の動作確認とサンプル | `sandbox/`(このリポジトリの中。配らない。`.proto` のパッケージは `sandbox.v1`、エラーの domain は `sandbox`) |
| 基盤の予約エラーの domain | `voltech`(アプリは使えない。[エラー](../api/errors.md)) |

- **リポジトリ名に、サーバーの言語(Rust)や Unity を入れない**。Go 版のサーバーを足す予定があり、言語を入れると後で名前が合わなくなる。使っている技術は、GitHub の説明文とトピックで伝える
- サーバーの言語を増やすときは、リポジトリを分けずに、基盤のリポジトリの中に `server/<言語>/` のフォルダを足す
- 基盤はパッケージとして配り、アプリは自分のプロジェクトから参照する。サンプルは基盤と同じリポジトリの sandbox に置く。理由と運用は [運用(パッケージとサンプル)](operations.md)
- **Rust のクレートは、機能ごとに分けたまま、頭に `voltech-grpc-` を付ける**。アプリは使う機能のクレートだけを依存に入れる。DB に残る名前(マイグレーションの履歴のテーブル名 `_auth_migrations` など)は、クレート名を変えても変えない
- `.proto` のパッケージ名は、基盤のものをすべて `voltech.v1` にまとめ、ファイル名で機能を分ける(`voltech/v1/device.proto`・`auth_options.proto`)。アプリの `.proto` は `app.v1` のように、アプリの名前空間に置く
  - `.proto` のパッケージ名は gRPC のメソッドのパス(`/voltech.v1.DeviceService/Register` など)に入る。言語の違うサーバーでも同じクライアントでつながるように、全言語で同じにする。リリース後に変えると古いアプリと通信できなくなるので、最初のリリースの前に決めた
  - `.proto` のパッケージ名は、通信のパスやログに毎回出るので短くする。コードの名前は `option csharp_namespace = "VolTech.V1";` のように指定して、長いままにする
- クライアントの基盤はパッケージなので、サーバーの言語を増やしても重複しない

## 実装で決めたこと

- エディタで通信を見るウィンドウの名前は **gRPC Monitor** にし、メニューは `Window > VolTech gRPC > gRPC Monitor` にする(2026-10-11 に Request Viewer から変えた)。リクエストだけでなく、レスポンス・トレーラー・エラーまで、エディタと実機の通信を流れで見るので、Request と Viewer では役割より狭い。映るのは gRPC の通信だけなので Network とは付けない(Unity の Network Profiler などとも紛れる)。ドッキングしたときにタブ名だけで何の道具か分かるように、gRPC を入れる
- samples の `Api` のアセンブリに混ざっていた基盤の部分は、パッケージにするときに分けた。認証(`DeviceTokenProvider` など)は `VolTech.Client.Auth`、Unity 用の層(`ApiChannel` など)は `VolTech.Client.Unity`、基盤の `.proto` の生成コードは `VolTech.V1`、エラーの既定の action の表は `VolTech.Client`(いまは `ErrorPolicyProvider`。アプリの `errors.toml` から生成した一覧を受け取る。[エラー](../api/errors.md))
- パッケージのテストのアセンブリは `VolTech.Client.Tests.EditMode` にする(`Tests.EditMode` のままだと、利用側の同じ名前の asmdef とぶつかる)
