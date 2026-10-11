# C# の生成に使うツールの準備と、生成の関数。基盤の Makefile と、アプリの Makefile が読む。
# ツールは make を実行するフォルダの .tools に入れる(基盤の版は、このファイルがある基盤のタグで決まる)。
#
# 使い方(アプリ):
#   proto-gen: voltech-tools
#   	$(PROTO_ENV) buf lint
#   	$(call voltech_proto_gen)
#   errors-gen: voltech-tools
#   	$(call voltech_errors_gen,errors.toml,client/Assets/Generated/Errors/AppErrors.cs,App.V1,AppErrors)

VOLTECH_ROOT := $(patsubst %/,%,$(dir $(patsubst %/,%,$(dir $(lastword $(MAKEFILE_LIST))))))
include $(VOLTECH_ROOT)/mk/env.mk

VOLTECH_TOOLS := .tools
# buf・protoc・grpc_csharp_plugin と、自作の protoc プラグインを PATH に足す
PROTO_ENV := PATH="$$PWD/$(VOLTECH_TOOLS)/bin:$$PWD/$(VOLTECH_TOOLS)/voltech-grpc:$$PATH"

# 読み込んだ側の既定のターゲット(引数なしの make)を変えない
VOLTECH_PREV_GOAL := $(.DEFAULT_GOAL)

.PHONY: voltech-tools proto-tools proto-plugin

voltech-tools: proto-tools proto-plugin ## 生成に使うツールと protoc プラグインを入れる

proto-tools: ## .proto のツール(buf・protoc・grpc_csharp_plugin)を .tools/bin に入れる
	$(BASH) $(VOLTECH_ROOT)/scripts/proto-tools.sh $(VOLTECH_TOOLS)/bin

proto-plugin: ## 自作の protoc プラグイン(protoc-gen-voltech-grpc)をビルドして .tools/voltech-grpc に入れる
	"$(DOTNET)" publish $(VOLTECH_ROOT)/tools/protoc-gen-voltech-grpc/src -c Release -o $(VOLTECH_TOOLS)/voltech-grpc --nologo -v quiet

.DEFAULT_GOAL := $(VOLTECH_PREV_GOAL)

# .proto から C# を生成する。$(1): buf generate に渡す引数(--template・--path など。無ければ buf.gen.yaml の全部)
voltech_proto_gen = $(PROTO_ENV) buf generate $(1)

# errors.toml から C# のエラーコードを生成する。
# $(1): errors.toml、$(2): 出力する .cs、$(3): 名前空間、$(4): クラス名、$(5): ほかの引数(基盤の予約エラーは --reserved)
voltech_errors_gen = $(PROTO_ENV) protoc-gen-voltech-grpc errors $(1) $(2) --namespace $(3) --class $(4) $(5)
