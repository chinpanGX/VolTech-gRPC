---
title: "protoc プラグインの中身"
status: current
area: dev
sections: []
related: [api/codegen, guide/tools, dev/code-map]
proto: []
code: [Tools/protoc-gen-voltech-grpc]
updated: 2026-10-10
---

# protoc プラグインの中身

`protoc-gen-voltech-grpc`(`Tools/protoc-gen-voltech-grpc/`)を直す人向け。使い方は [ツール](../guide/tools.md)、設計の理由は [コード生成](../api/codegen.md)。

- .NET 10 のコンソールアプリ。`dotnet publish` で `.tools/voltech-grpc` に入れ、`buf.gen.yaml` の `local: protoc-gen-voltech-grpc` から呼ぶ(`make proto-plugin`)
- 依存は `Google.Protobuf` だけ(入出力の型 `CodeGeneratorRequest`・`CodeGeneratorResponse` を含む)
- 生成するコードは、クライアントの基盤(`VolTech.Grpc.Client` の `ApiCore`・`RpcSettings`・`ApiResult`)の API に合わせる。基盤の API を変えたら、プラグインも同時に直す

## ファイル

| ファイル | 役割 |
|---|---|
| `src/Program.cs` | 入口。標準入力の `CodeGeneratorRequest` を `Generator.Generate` に渡し、標準出力に `CodeGeneratorResponse` を書く。`GeneratorException` は `response.Error` にして返す(buf がエラーとして表示する)。標準出力は応答にしか使えないので、ログを書かない |
| `src/Generator.cs` | 生成の本体。`.proto` のファイルごとに `<ファイル名>Api.cs` を作る。ラッパー(`WriteWrapper`)・RPC のパスの定数(`WriteMethods`)・共通レスポンス項目の目印のインターフェース(`WriteInterface`)を書く。共通レスポンス項目のフィールド名と interface の名前の表(`commonResponseFields`)もここにある |
| `src/GeneratorParameters.cs` | オプション(`package=`・`error_code=`)を読む |
| `src/ProtoOptions.cs` | `.proto` のオプションを読む。`idempotency_level`(標準)と `skip_auth`(基盤のカスタムオプション。タグ番号 50002 で読む) |
| `src/TypeIndex.cs` | リクエストに含まれる全ファイルのメッセージと enum を、完全名(`.app.v1.SignInRequest`)から引く。C# の型名は `global::` 付きで返す |
| `src/CSharpNames.cs` | `.proto` の名前を、protoc 組み込みの C# 生成と同じ規則で C# の名前にする(名前空間・ファイル名・プロパティ名)。生成したコードは protoc と grpc_csharp_plugin が作った型を参照するので、規則をそろえる |
| `src/Comments.cs` | `.proto` のコメント(`SourceCodeInfo`)を引き、生成するコードの XML ドキュメントにする |
| `src/CodeWriter.cs` | インデントを付けながら C# を書く小さな道具 |
| `tests/` | xUnit のテスト。`Protos.cs` が `FileDescriptorProto` を手で組み立て、`GeneratorTests.cs` が生成結果の文字列を確かめる |

## 決まりごと

- **`skip_auth` はタグ番号で読む**。アプリの `.proto` は `voltech/v1/auth_options.proto` を import しないので、リクエストに拡張の定義が入っていないことがある。番号は `auth_options.proto` と同じにする(`ProtoOptions.SkipAuthFieldNumber`)
- **生成するファイルの using はアルファベット順**にする。`make client-lint` は生成コードを見ないので、ずれても気づけない。テストで確かめている
- 生成するものが無いファイル(メッセージだけの `.proto` など)は、ファイルを作らない
- 共通レスポンス項目の目印は、項目の型を定義したファイルに interface を、項目を持つレスポンスのファイルに `partial class` を出す

## 直したとき

1. `make plugin-test` でテストを流す
2. `make proto-gen` で生成し直し、生成コードの差分を確かめる(意図しない差分が無いか)
3. Unity でコンパイルが通ることを確かめる(Unity のエディタを開いているなら `unity recompile`)
4. 生成するものの形を変えたら、[コード生成](../api/codegen.md) と [ツール](../guide/tools.md) を直す
