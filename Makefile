# 開発用のコマンド
.PHONY: help db-up db-down proto-tools proto-plugin plugin-test proto-lint proto-format proto-breaking proto-gen build fmt fmt-check lint client-fmt client-lint wiki-gen wiki-lint wiki-affected sandbox-proto-gen test test-rust

ifeq ($(OS),Windows_NT)
# PowerShell から make を実行しても、レシピは Git Bash で動かす(既定の cmd.exe では動かないため)。
# Git を別の場所に入れたときは、環境変数 GIT_BASH でパスを渡す
GIT_BASH ?= C:/Program Files/Git/bin/bash.exe
SHELL := $(GIT_BASH)
# 単なる bash だと、PATH 上の WSL の bash.exe が起動することがあるので、パスで指定する
BASH := "$(GIT_BASH)"
else
BASH := bash
endif

# .proto のツールは scripts/proto-tools.sh が .tools/bin に、自作のプラグインは make proto-plugin が .tools/voltech-grpc に入れたものを使う
PROTO_ENV := PATH="$$PWD/.tools/bin:$$PWD/.tools/voltech-grpc:$$PATH"
PLUGIN_DIR := tools/protoc-gen-voltech-grpc

RUST_MANIFESTS := server/rust/Cargo.toml
UNITY_SLN := sandbox/UnityProject/UnityProject.sln

# 説明は各ターゲットの行の「## 」の後ろに書く。
# Windows 版の make は、レシピのコマンドを ANSI コードページ経由で渡すので、日本語を echo すると文字化けする。
# そのため説明はレシピに書かず、Makefile 自体から読み出して表示する
help: ## コマンドの一覧
	@grep -E '^[a-z-]+:.*## ' Makefile | sed -E 's/^([a-z-]+):.*## (.*)/\1|\2/' | awk -F'|' '{ printf "%-16s %s\n", $$1, $$2 }'

db-up: ## MySQL を起動
	docker compose up -d --wait mysql

db-down: ## MySQL を停止
	docker compose down

proto-tools: ## .proto のツール(buf・protoc・grpc_csharp_plugin)を .tools/bin に入れる
	$(BASH) scripts/proto-tools.sh

proto-lint: proto-tools ## buf lint
	$(PROTO_ENV) buf lint

proto-format: proto-tools ## buf format -w
	$(PROTO_ENV) buf format -w

proto-breaking: proto-tools ## buf breaking(main ブランチと比較)
	$(PROTO_ENV) buf breaking --against '.git#branch=main'

proto-plugin: ## 自作の protoc プラグイン(protoc-gen-voltech-grpc)をビルドして .tools/voltech-grpc に入れる
	dotnet publish $(PLUGIN_DIR)/src -c Release -o .tools/voltech-grpc --nologo -v quiet

plugin-test: ## 自作の protoc プラグインのテスト
	dotnet test $(PLUGIN_DIR)/tests --nologo

proto-gen: proto-lint ## 基盤の .proto から C# を生成(UPM パッケージの Runtime/Generated)
	$(PROTO_ENV) buf generate --path server/rust/auth/proto/voltech --path server/rust/error/proto/voltech

sandbox-proto-gen: proto-lint proto-plugin ## sandbox の .proto から C# を生成(sandbox の Unity プロジェクト)
	$(PROTO_ENV) buf generate --template buf.gen.sandbox.yaml --path sandbox/proto/sandbox

build: ## cargo build
	$(foreach m,$(RUST_MANIFESTS),cargo build --manifest-path $(m) &&) true

fmt: ## rustfmt で整形
	$(foreach m,$(RUST_MANIFESTS),cargo fmt --manifest-path $(m) --all &&) true

fmt-check: ## rustfmt で整形済みか確認
	$(foreach m,$(RUST_MANIFESTS),cargo fmt --manifest-path $(m) --all --check &&) true

lint: ## Clippy(警告はエラー扱い)
	$(foreach m,$(RUST_MANIFESTS),cargo clippy --manifest-path $(m) --workspace --all-targets -- -D warnings &&) true

client-fmt: ## dotnet format でクライアントを直す
	dotnet format $(UNITY_SLN) --severity warn

client-lint: ## クライアントの書き方の違反を確認
	dotnet format $(UNITY_SLN) --severity warn --verify-no-changes

test-rust: db-up ## Rust のテスト(MySQL を使う)
	$(foreach m,$(RUST_MANIFESTS),cargo test --manifest-path $(m) &&) true

wiki-gen: ## docs/wiki の API の表を .proto のコメントから生成し直す
	python scripts/wiki_gen.py

wiki-lint: ## docs/wiki のリンク切れ・frontmatter・API の表の生成し忘れを確認
	python scripts/wiki_gen.py --check && python scripts/wiki_lint.py

wiki-affected: ## 変わったファイルから直すべき docs/wiki のページを探す(BASE=main でブランチ全体、FILES="a b" でファイルを直接)
	python scripts/wiki_affected.py $(if $(FILES),--files $(FILES),$(BASE))

# TODO: Unity の EditMode テスト(クライアントの本体を含む)を足す
test: test-rust plugin-test wiki-lint ## プッシュ前のテスト一式
