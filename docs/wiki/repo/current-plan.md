---
title: "当面の実装方針"
status: current
area: repo
related: [repo/operations, repo/naming, repo/folder-layout, api/errors]
proto: []
updated: 2026-10-11
---

# 当面の実装方針

> **2026-10-10 に分割を終えた。** 基盤は grpc-foundation-samples(タグ `pre-split`)から、このリポジトリ(VolTech-gRPC)に新しい履歴でコピーした。手順 1(パッケージの形にする)はコピー先で行い、手順 2 は履歴を引き継がないコピーに変えた(samples の履歴は samples に残す)。いまのフォルダは [コードの地図](../dev/code-map.md) にある。
>
> **2026-10-11 に、samples のガチャを sandbox に移した。** アプリは基盤をパッケージとして参照し、サンプルは基盤と同じリポジトリに置く([運用](operations.md))。grpc-foundation-samples は使わない。下は分ける前の方針と、samples の中にあった頃の形で、経緯として残す(下の「このリポジトリ」は samples を指す)。

まずは1つのリポジトリ(`grpc-foundation-samples`)で、基盤(パッケージ)・テンプレート・サンプルの中身を実装する。動かし方は [環境構築とよく使うコマンド](../ops/setup.md) にある。

基盤は、形が固まったら `voltech-grpc` のリポジトリに分ける。このリポジトリは、基盤を使うサンプル兼テンプレートとして残す(名前は [リポジトリ名と命名](naming.md))。移す時期は [未確定・保留](../open-questions.md) の #17。

基盤の移し方:

1. **このリポジトリで、基盤をパッケージの形にする**。Unity は基盤を埋め込みパッケージ(`Packages/com.chinpangx.voltech.grpc/`、`package.json` 付き)に移す。サーバーの `framework/` のクレートと、基盤の `.proto` は、今の形のまま単独でビルドできる状態を保つ
2. **履歴ごと `voltech-grpc` に移す**(`git subtree split` か `git filter-repo`)
3. **サンプルから参照する**。Cargo は git の依存(`tag` を指定)、UPM は git の URL(`?path=` とタグ)にする。基盤を同時に直すときは、手元のパスに差し替える([基盤の版を上げる・基盤を一緒に直す](../guide/upgrade.md))。サンプルの buf から基盤の `.proto` を参照する方法(#3)は、このときに決め直す

アプリは、このリポジトリ(サンプル兼テンプレート)を clone して作り、基盤はパッケージとして参照する(分けた後は、アプリが自分のプロジェクトから基盤を参照する形にした。[運用](operations.md))

- 後で分けるときに切り出しやすいように、**アプリごとに変更しないもの(`framework/`)と、変更するもの(`app/`)をフォルダで分ける**
- `framework/` のクレートは、**実装する機能ごとに分け、`voltech-grpc-<機能>` の名前を付ける**(`voltech-grpc-auth`・`voltech-grpc-error` など)。フォルダは機能の名前だけにする(`framework/auth/`)
- 基盤の `.proto` のパッケージは `voltech.v1` にまとめ、ファイル名で機能を分ける。`.proto` は、その機能のクレートに同梱する
- Unity では、基盤の本体を `VolTech.Client` のアセンブリ(`Assets/Scripts/VolTech.Client/`)に置く
- `framework/` は `app/` に依存しない。それだけでビルド・テストでき、そのまま別のアプリやリポジトリに持っていける状態を保つ
- アプリとのつなぎ目は trait(`PlayerResolver` など)にし、`app/` 側で実装する。アプリの都合で `framework/` を直したくなったら、直接変えずに差し替え口を足す

```
grpc-foundation-samples/
├── Server/                     Cargo ワークスペース(1つ)
│   ├── framework/              アプリごとに変更しない
│   │   ├── auth/               voltech-grpc-auth: デバイス登録・トークンの発行・認証。.proto(voltech/v1/device.proto・auth_options.proto)とマイグレーションを同梱する
│   │   ├── error/              voltech-grpc-error: エラーのレスポンスの作り方・予約エラーコード。action の .proto(voltech/v1/error_options.proto)を同梱する
│   │   ├── status/             voltech-grpc-status: サーバーの状態(メンテナンス・強制アップデート・日付変更)の判定と、サーバー時刻・日付のヘッダー
│   │   └── transaction/        voltech-grpc-transaction: トランザクションと冪等性(RequestID で二重実行を防ぐ)。マイグレーションを同梱する
│   └── app/                    アプリごとに変更する
│       ├── proto/              Shared/proto から生成したコード
│       ├── player/ gacha/      ドメインのクレート
│       ├── api/                gRPC のハンドラー
│       ├── server/             バイナリ
│       ├── migrations/         アプリのテーブルのマイグレーション
│       └── config/             server_status.toml
├── Unity/UnityProject/         Unity プロジェクト
│   └── Assets/Scripts/
│       ├── VolTech.Client/          クライアントの基盤の本体(ApiCore など。Unity に依存しない)
│       ├── VolTech.Client.Tracing/  通信の記録(エディタと開発ビルドだけ)
│       ├── VolTech.Client.Editor/   gRPC Monitor(エディタだけ)
│       ├── Api/                          認証・Unity 用の層・生成コード・アプリの反映処理(基盤とアプリが混ざっている)
│       └── Sample/                       サンプルの画面
├── Shared/proto/               アプリの .proto(error_code.proto など)
├── docs/wiki/           設計(この wiki)
├── tools/protoc-gen-voltech-grpc/ ラッパーを生成する protoc プラグイン
└── buf.yaml / buf.gen.yaml / docker-compose.yml / Makefile
```

[フォルダ構成](folder-layout.md) の形との対応:

| [フォルダ構成](folder-layout.md) の形 | 当面の置き場所 |
|---|---|
| 基盤の Rust のクレート(`voltech-grpc-auth` など) | `server/rust/<機能>/`(`auth` など) |
| 基盤の `.proto`(`voltech.v1`) | 機能のクレートの中(`framework/auth/proto/voltech/v1/` など) |
| サーバーの基盤のうち、アプリごとに変えないもの(トランザクションなど) | `server/rust/<機能>/`。機能ごとのクレートにする(`voltech-grpc-transaction` など) |
| テンプレートの `Server/crates/*` | `Server/app/*` |
| テンプレートの `Client/` | `Unity/UnityProject/` |
| protoc プラグイン(`protoc-gen-voltech-grpc`) | `tools/protoc-gen-voltech-grpc/` |
| クライアントの基盤(UPM の `com.chinpangx.voltech.grpc`) | 本体は `Assets/Scripts/VolTech.Client/`。認証と Unity 用の層は、まだ `Api` のアセンブリにあり、パッケージの形にするときに分ける |

実装の中で決めたこと:

- カスタムオプションは機能の `.proto` に置く。`ErrorAction` と `default_action` は `voltech/v1/error_options.proto`、`skip_auth` は `voltech/v1/auth_options.proto`。アプリの `error_code.proto` は `voltech/v1/error_options.proto` を import する(いまはエラーを `errors.toml` で定義するので、この2つは無い。[エラー](../api/errors.md))
- パッケージが返す予約エラーコードは、`voltech-grpc-error` クレートにまとめる(いまは `voltech_grpc_error::reserved`。`errors.toml` から生成する)
- `player_id` は ULID にする。DB には `BINARY(16)`、API(`.proto`)とクライアントには 26 文字の文字列で持つ。連番と違い、他人の ID や登録者数を推測されない。基盤の `PlayerId` の型も ULID(`ulid::Ulid`)にする
- buf からパッケージの `.proto` は、手元のパス(`buf.yaml` の `modules`)で参照する([未確定・保留](../open-questions.md) #3)
- Rust の生成は、使う側の `build.rs` で protox(Rust 製の protobuf コンパイラ)を使って行い、生成コードはコミットしない。protoc のインストールは要らない
- `.proto` のパッケージ名(`voltech.v1`)と、DB に残る名前(マイグレーションの履歴のテーブル名 `_auth_migrations` など)は、リリース後には変えられないので、最初のリリースまでに確定させる。フォルダ名・クレート名・名前空間は、後からでも変えられる
