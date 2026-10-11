---
name: deps-update
description: 基盤の依存パッケージ(Unity の package.json の UPM・NuGet、Rust のクレート、protoc プラグインの NuGet、buf・Grpc.Tools)の新しい版を調べ、一緒に上げないと壊れる組み合わせごとに上げて、生成し直し・テスト・CHANGELOG・wiki まで済ませる。「依存を更新」「パッケージを上げて」「アップデートチェック」「新しい版はある？」「deps-update」と言われたとき、Unity のエディタの版を上げたとき、依存の版のずれでテストが落ちたときに使う。
---

# 基盤の依存の更新

基盤の依存には、**一緒に上げないと壊れる組み合わせ**がある。1つだけ上げると、コンパイルは通るのに動かない(例: Google.Protobuf だけを上げると、Unity のエディタの中で業務エラーがすべて不明なエラーになる)。組み合わせの決まりは `scripts/deps.py` の `check` が正で、`make deps-check` で確かめる。

## 引数

- なし: 新しい版を調べて一覧を見せ、どれを上げるかを人に確かめてから上げる
- パッケージ名(1つ以上): そのパッケージと、組になるものを上げる
- `check`: 調べて報告するだけで、何も上げない

## 組み合わせ

| 組 | 一緒に直すところ | 決まり |
|---|---|---|
| Protobuf | `Unity/com.chinpangx.voltech.grpc/package.json` の `org.nuget.google.protobuf`・`org.nuget.google.api.commonprotos`、`tools/protoc-gen-voltech-grpc/src/ProtocGenVoltechGrpc.csproj` の `Google.Protobuf` | Google.Protobuf は、CommonProtos が依存する版にそろえる(新しい Protobuf を使いたいときは、その版を参照する CommonProtos が出るまで待つ)。プラグインも同じ版 |
| gRPC(C#) | `package.json` の `org.nuget.grpc.net.client`・`org.nuget.grpc.core.api`、`scripts/proto-tools.sh` の `GRPC_TOOLS_VERSION` | 3つとも同じ版 |
| buf | `scripts/proto-tools.sh` の `BUF_VERSION` | 単独で上げてよい |
| UniTask・YetAnotherHttpHandler・System.IO.Pipelines | `package.json` | 単独で上げてよい |
| tonic・prost | `server/rust/Cargo.toml` と `sandbox/server/Cargo.toml` の `tonic`・`tonic-*`・`prost`・`prost-types`(と、それを使う `protox`) | 一族は同じ版。tonic と prost は対応する版の組で上げる(tonic の CHANGELOG で確かめる) |
| その他の Rust のクレート | `server/rust/Cargo.toml`、`sandbox/server/Cargo.toml` | sandbox は基盤と同じ版にする |
| protoc プラグインのテストの道具(xunit など) | `tools/protoc-gen-voltech-grpc/tests/ProtocGenVoltechGrpc.Tests.csproj` | 単独で上げてよい |

## 手順

1. **いまの状態を確かめる**。リポジトリのルートで実行する

   ```
   make deps-check      # 組み合わせの決まり(ネットワークを使わない)。error が 0 でなければ、先にそれを直す
   make deps-outdated   # 新しい版の一覧(* が付いたものに新しい版がある)
   ```

   Rust は、`Cargo.toml` の要求の範囲内の更新(`0.14` の中の `0.14.6` など)には印が付かない。範囲内の更新は `cargo update --manifest-path <Cargo.toml>` で `Cargo.lock` だけが変わる。

2. **上げるものを決める**。引数が無ければ、一覧と組み合わせを見せて、どれを上げるかを人に確かめる。major(0.x は minor)を上げるものは、リリースノートで破壊的な変更を読んでから上げる。`deps-outdated` の備考に「〜までしか上げない」とあるものは、その版より上げない

3. **組ごとに上げる**。上の表の「一緒に直すところ」を全部直す。組をまたいで同時に上げない(壊れたときにどれが原因か分からなくなる)

4. **生成し直す・解決し直す**
   - Unity の依存を変えたら: sandbox の Unity プロジェクトでパッケージを解決し直し、`sandbox/UnityProject/Packages/packages-lock.json` を更新する。エディタが開いていれば `unity command package_resolve`(Unity CLI)、開いていなければ Unity で `sandbox/UnityProject` を開く
   - Grpc.Tools・buf・Google.Protobuf を変えたら: `make proto-gen sandbox-proto-gen` で C# を生成し直し、差分をコミットに含める(ツールは `make` が入れ直す)
   - Rust を変えたら: `cargo update --manifest-path server/rust/Cargo.toml` と `cargo update --manifest-path sandbox/server/Cargo.toml` で `Cargo.lock` を更新する

5. **確かめる**。直した組に関係するものを全部流す

   | 直したもの | 確かめること |
   |---|---|
   | どれでも | `make deps-check` の error が 0 |
   | Rust | `make fmt` → `make lint` → `make test-rust`(MySQL を使う) |
   | protoc プラグイン・Google.Protobuf・Grpc.Tools | `make plugin-test`、生成し直したコードの差分 |
   | Unity の依存 | sandbox の EditMode テストをすべて流す(`unity command run_tests --mode EditMode`)。`SandboxServerTests` は起動したサーバーにつなぐので、先に `make sandbox-server` を起動しておく |

   - Windows で `dotnet` が Unity 同梱の .NET 8 SDK になり、`make plugin-test` が NETSDK1045 で止まることがある(Unity 6000.6 が PATH の前に DotNetSdk を足すため)。そのときは PATH の先頭に `C:\Program Files\dotnet` を置いて流す
   - テストが落ちたら、その組を元に戻すか、原因を直してから次の組に進む

6. **記録する**
   - `CHANGELOG.md` の `[Unreleased]` に、上げたものと、利用側がすること(Unity で開き直す、など)を書く
   - 版を書いている wiki のページを直す(ツールの版は `docs/wiki/appendix/setup.md`、Unity の依存の決まりは `docs/wiki/client/structure.md`)。`/wiki-update` の手順で、ほかに直すページが無いかも確かめる
   - **新しい組み合わせの決まりを見つけたら**(一緒に上げないと壊れたもの)、`scripts/deps.py` の `check` に確かめる処理を足し、上の「組み合わせ」の表にも足す

7. **報告する**。上げたもの(前の版 → 新しい版)、上げなかったものとその理由、流したテストと結果を短く挙げる。コミットは組ごとに分ける(`chore(deps): ...`)

## Unity のエディタの版を上げたとき

エディタは自分でも DLL(Google.Protobuf・Grpc など)を同梱していて、パッケージの DLL と食い違うことがある。エディタを上げたら、依存を変えていなくても手順 5 の Unity の確かめを行う。

- 落ちたら、どの版の DLL が読み込まれているかを確かめる(`unity command eval` で `AppDomain.CurrentDomain.GetAssemblies()` を見る)。同じ名前で版の違うアセンブリが2つ読み込まれていたら、版のずれが原因
- `docs/wiki/appendix/setup.md` の Unity の版を直す

## 注意

- 基盤の依存は、基盤を参照するアプリにそのまま入る。上げたら CHANGELOG に必ず書く(アプリはタグを上げるときに CHANGELOG を読む)
- `package.json` の `unity`(対応する Unity の最低の版)は、エディタを上げただけでは変えない。新しいエディタでしか使えない API を使ったときに上げる
- 日本語で書く。Python などで日本語を含む文字列を置き換えるときは UTF-8 で読み書きする
