---
title: "始め方"
status: current
area: guide
related: [guide/connect, guide/add-api, guide/upgrade, guide/roles, repo/operations, api/errors]
proto: []
code: [mk, local.mk.example, scripts/proto-tools.sh, tools/protoc-gen-voltech-grpc/src, sandbox/server/Cargo.toml, sandbox/server/proto/build.rs, sandbox/UnityProject/Packages/manifest.json, sandbox/errors.toml, buf.gen.sandbox.yaml, sandbox/server/server/src/lib.rs, sandbox/UnityProject/Assets/Scripts/Sample/SampleLifetimeScope.cs, .env.example]
updated: 2026-10-11
---

# 始め方

アプリは自分のプロジェクト(Unity プロジェクトと、Rust のサーバー)を作り、基盤(このリポジトリ、VolTech gRPC)をパッケージとして参照する。サーバーは Cargo の `git` 依存、クライアントは UPM の git URL で、どちらもリポジトリの URL とタグを指定する。C# の生成に使う protoc プラグインは、アプリの Makefile が同じタグの基盤を取ってきてビルドする。なぜこの形にしたかは [運用(パッケージとサンプル)](../repo/operations.md) にある。

アプリのフォルダは、次の形を基本にする(sandbox も同じ形。`sandbox/UnityProject`・`sandbox/server`・`sandbox/proto`・`sandbox/errors.toml`)。

```
<アプリ>/
├── client/          Unity プロジェクト(Packages/manifest.json で基盤を参照する)
├── server/          Rust のワークスペース(Cargo.toml で基盤を参照する)
├── proto/           アプリの .proto
├── errors.toml      アプリの業務エラー
├── buf.yaml / buf.gen.yaml
└── Makefile         C# の生成(下の「3. C# を生成する」)
```

組み込み方の実例は、このリポジトリの sandbox にある。基盤の機能を1つずつ試す RPC と、ガチャのサンプル(タイトル → サインアップ → ガチャ)が入っている。

アプリを作る流れは、このページ(基盤を参照し、生成の準備をする) → [クライアントとサーバーをつなぐ](connect.md) → [API を足す](add-api.md) の順。

## まず動くものを見る(sandbox)

アプリを作る前に、このリポジトリの sandbox で、基盤が動くところを見られる。Docker・Rust・Unity・GNU Make が要る(Windows では Git for Windows も)。

1. このリポジトリを clone し、`.env.example` を `.env` にコピーして、`make sandbox-server` でサーバーを起動する(MySQL を Docker で起動し、`localhost:50051` で待ち受ける)
2. Unity で `sandbox/UnityProject` を開き、`Assets/Scenes/Sample.unity` を Play する。Start を押すと、デバイス登録 → サインイン(初回はサインアップ)→ ガチャの画面に進む
3. `Window > VolTech gRPC > gRPC Monitor` で、通信ごとのヘッダーと Body を見られる([通信を見る・調べる](debugging.md))

サーバーの状態(メンテナンス・強制アップデート・日付が変わる時刻)は `sandbox/server/config/server_status.toml` で決める。サーバーが 5 秒ごとに読み直すので、書き換えれば再起動しなくても効く([サーバーの状態](../server/status.md))。


## 1. 基盤を参照する

サーバー(アプリの `Cargo.toml`)。クレート名で探されるので、ワークスペースの中の場所は書かなくてよい。

```toml
[workspace.dependencies]
voltech-grpc-auth = { git = "https://github.com/chinpanGX/VolTech-gRPC", tag = "v0.2.0" }
voltech-grpc-error = { git = "https://github.com/chinpanGX/VolTech-gRPC", tag = "v0.2.0" }
voltech-grpc-error-build = { git = "https://github.com/chinpanGX/VolTech-gRPC", tag = "v0.2.0" }
voltech-grpc-log = { git = "https://github.com/chinpanGX/VolTech-gRPC", tag = "v0.2.0" }
voltech-grpc-status = { git = "https://github.com/chinpanGX/VolTech-gRPC", tag = "v0.2.0" }
voltech-grpc-transaction = { git = "https://github.com/chinpanGX/VolTech-gRPC", tag = "v0.2.0" }
```

クライアント(Unity の `Packages/manifest.json`)。パッケージが依存する UniTask・YetAnotherHttpHandler(OpenUPM)と gRPC の DLL(Unity NuGet)を解決できるように、`scopedRegistries` も足す。

```json
{
  "dependencies": {
    "com.chinpangx.voltech.grpc": "https://github.com/chinpanGX/VolTech-gRPC.git?path=/Unity/com.chinpangx.voltech.grpc#v0.2.0"
  },
  "scopedRegistries": [
    { "name": "Unity NuGet", "url": "https://unitynuget-registry.openupm.com", "scopes": ["org.nuget"] },
    { "name": "OpenUPM", "url": "https://package.openupm.com", "scopes": ["com.cysharp.unitask", "com.cysharp.yetanotherhttphandler"] }
  ]
}
```

- タグは Cargo と UPM で同じ値を使う(リリースの手順は [未確定・保留](../open-questions.md) #16)
- 版の上げ方と、基盤とアプリを同時に直すときの手順は [基盤の版を上げる・基盤を一緒に直す](upgrade.md)

## 2. アプリの .proto とエラーを用意する

- アプリの `.proto` を書く([API を足す](add-api.md))。アプリの `.proto` は基盤の `.proto` を import しない
- アプリの業務エラーを、アプリの `errors.toml` に書く。domain はアプリの名前にする(`voltech` は使えない。[エラー](../api/errors.md))
- Rust は、アプリの `build.rs` で `.proto` と `errors.toml` から生成する(`voltech_grpc_error_build::compile`)。sandbox の `sandbox/server/proto/build.rs` が実例
- C# は、`buf generate` で `.proto` から、`protoc-gen-voltech-grpc errors` で `errors.toml` から生成し、生成したコードはコミットする(下の「3. C# を生成する」)

## 3. C# を生成する

protoc プラグイン(`protoc-gen-voltech-grpc`)はパッケージとして配らない。アプリの Makefile が、`manifest.json`・`Cargo.toml` と同じタグの基盤から、生成に使うフォルダ(`mk/`・`scripts/`・`tools/protoc-gen-voltech-grpc/`)だけを `.tools/` に取ってくる。そこにある `mk/codegen.mk` を読み込めば、ツール(buf・protoc・grpc_csharp_plugin)とプラグインを同じ版で入れるコマンドと、生成の関数が使える。`.tools/` と `local.mk` はコミットしない(`.gitignore` に足す)。要るのは Git・GNU Make・.NET SDK(10 以上)と、Windows では Git for Windows。

`Makefile`(例):

```make
# 基盤のタグ。client/Packages/manifest.json・server/Cargo.toml のタグと同じにする
VOLTECH_TAG := v0.2.0
VOLTECH_REPO := https://github.com/chinpanGX/VolTech-gRPC
VOLTECH_ROOT := .tools/voltech-grpc-src/$(VOLTECH_TAG)

.PHONY: proto-gen errors-gen

# 生成のツールの準備と、利用者ごとの設定(.NET・Git Bash)を基盤から読む
include $(VOLTECH_ROOT)/mk/codegen.mk

# 基盤が無ければ取ってくる(make は読み込むファイルを作ってから読み直す)。
# Git Bash の設定を読む前に動くので、Windows の cmd でも動く書き方にする。
# 使うフォルダだけを取る(sandbox の Unity プロジェクトは、パスが長く Windows で展開できないことがあるため取らない)
$(VOLTECH_ROOT)/mk/codegen.mk:
	git clone --quiet --depth 1 --filter=blob:none --sparse --branch $(VOLTECH_TAG) $(VOLTECH_REPO) $(VOLTECH_ROOT)
	git -C $(VOLTECH_ROOT) sparse-checkout set mk scripts tools/protoc-gen-voltech-grpc

proto-gen: voltech-tools
	$(PROTO_ENV) buf lint
	$(call voltech_proto_gen)

errors-gen: voltech-tools
	$(call voltech_errors_gen,errors.toml,client/Assets/Generated/Errors/AppErrors.cs,App.V1,AppErrors)
```

- `voltech-tools`・`PROTO_ENV`・`voltech_proto_gen`・`voltech_errors_gen` は `mk/codegen.mk` にある。基盤の Makefile も同じものを使う
- 利用者ごとに変わる設定(使う `dotnet`、Windows の Git Bash の場所)は、`make` を実行するフォルダの `local.mk` か環境変数で上書きする。書ける設定は基盤の `mk/env.mk`、例は `local.mk.example`。たとえば Unity の .NET 8 SDK が PATH で先に見つかる PC では、`local.mk` に `DOTNET := C:/Program Files/dotnet/dotnet.exe` と書く
- 取ってくる途中で失敗したときは、`.tools/voltech-grpc-src/` を消してから実行し直す

`buf.yaml`:

```yaml
version: v2
modules:
  - path: proto
lint:
  use:
    - STANDARD
```

`buf.gen.yaml`(sandbox の `buf.gen.sandbox.yaml` と同じ形。出力先と `package` をアプリに合わせる):

```yaml
version: v2
plugins:
  - protoc_builtin: csharp
    out: client/Assets/Generated/Messages
    opt:
      - base_namespace=
  - local: grpc_csharp_plugin
    out: client/Assets/Generated/Grpc
    opt:
      - no_server
      - base_namespace=
  - local: protoc-gen-voltech-grpc
    out: client/Assets/Generated/Wrappers
    opt:
      - package=app.v1
```

- 基盤のタグは `VOLTECH_TAG`・`manifest.json`・`Cargo.toml` の3か所でそろえる([基盤の版を上げる・基盤を一緒に直す](upgrade.md))
- Rust の生成にはコマンドは要らない(`build.rs` がビルドのときに生成する)

## 次に読むもの

- [クライアントとサーバーをつなぐ](connect.md) — サーバーとクライアントを組み立て、つながるのを確かめる
- [基盤とアプリの役割分担](roles.md) — アプリが作るもの・作らないもの
