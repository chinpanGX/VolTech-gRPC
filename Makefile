# 基盤の開発用のコマンド(基盤と sandbox)。アプリは使わない。
# C# の生成に使うツールの準備と、利用者ごとの設定(.NET・Git Bash)は mk/ にあり、アプリの Makefile も同じものを読む
include mk/codegen.mk

.PHONY: help db-up db-down plugin-test proto-lint proto-format proto-breaking proto-gen build fmt fmt-check lint wiki-gen wiki-lint wiki-affected deps-check deps-outdated sandbox-proto-gen errors-gen sandbox-errors-gen sandbox-server test test-rust

PLUGIN_DIR := tools/protoc-gen-voltech-grpc

RUST_MANIFESTS := server/rust/Cargo.toml sandbox/server/Cargo.toml
# エラーの定義ファイル(errors.toml)から生成する C#
RESERVED_ERRORS_CS := Unity/com.chinpangx.voltech.grpc/Runtime/Client/Errors/ReservedErrors.cs
SANDBOX_ERRORS_CS := sandbox/UnityProject/Assets/Generated/Errors/SandboxErrors.cs

# 説明は各ターゲットの行の「## 」の後ろに書く(mk/ のターゲットも出す)。
# Windows 版の make は、レシピのコマンドを ANSI コードページ経由で渡すので、日本語を echo すると文字化けする。
# そのため説明はレシピに書かず、Makefile 自体から読み出して表示する
help: ## コマンドの一覧
	@grep -hE '^[a-z-]+:.*## ' Makefile mk/codegen.mk | sed -E 's/^([a-z-]+):.*## (.*)/\1|\2/' | awk -F'|' '{ printf "%-16s %s\n", $$1, $$2 }'

db-up: ## MySQL を起動
	docker compose up -d --wait mysql

db-down: ## MySQL を停止
	docker compose down

proto-lint: proto-tools ## buf lint
	$(PROTO_ENV) buf lint

proto-format: proto-tools ## buf format -w
	$(PROTO_ENV) buf format -w

proto-breaking: proto-tools ## buf breaking(main ブランチと比較)
	$(PROTO_ENV) buf breaking --against '.git#branch=main'

plugin-test: ## 自作の protoc プラグインのテスト
	"$(DOTNET)" test $(PLUGIN_DIR)/tests --nologo

proto-gen: proto-lint ## 基盤の .proto から C# を生成(UPM パッケージの Runtime/Generated)
	$(call voltech_proto_gen,--path server/rust/auth/proto/voltech)

errors-gen: proto-plugin ## 基盤の予約エラー(server/rust/error/errors.toml)から C# を生成(UPM パッケージの ReservedErrors)
	$(call voltech_errors_gen,server/rust/error/errors.toml,$(RESERVED_ERRORS_CS),VolTech.Client,ReservedErrors,--reserved)

sandbox-proto-gen: proto-lint proto-plugin ## sandbox の .proto から C# を生成(sandbox の Unity プロジェクト)
	$(call voltech_proto_gen,--template buf.gen.sandbox.yaml --path sandbox/proto/sandbox)

sandbox-errors-gen: proto-plugin ## sandbox の業務エラー(sandbox/errors.toml)から C# を生成(sandbox の Unity プロジェクト)
	$(call voltech_errors_gen,sandbox/errors.toml,$(SANDBOX_ERRORS_CS),Sandbox.V1,SandboxErrors)

build: ## cargo build
	$(foreach m,$(RUST_MANIFESTS),cargo build --manifest-path $(m) &&) true

fmt: ## rustfmt で整形
	$(foreach m,$(RUST_MANIFESTS),cargo fmt --manifest-path $(m) --all &&) true

fmt-check: ## rustfmt で整形済みか確認
	$(foreach m,$(RUST_MANIFESTS),cargo fmt --manifest-path $(m) --all --check &&) true

lint: ## Clippy(警告はエラー扱い)
	$(foreach m,$(RUST_MANIFESTS),cargo clippy --manifest-path $(m) --workspace --all-targets -- -D warnings &&) true

test-rust: db-up ## Rust のテスト(MySQL を使う)
	$(foreach m,$(RUST_MANIFESTS),cargo test --manifest-path $(m) &&) true

wiki-gen: ## docs/wiki の API の表を .proto のコメントから生成し直す
	python scripts/wiki_gen.py

wiki-lint: ## docs/wiki のリンク切れ・frontmatter・API の表の生成し忘れを確認
	python scripts/wiki_gen.py --check && python scripts/wiki_lint.py

wiki-affected: ## 変わったファイルから直すべき docs/wiki のページを探す(BASE=main でブランチ全体、FILES="a b" でファイルを直接)
	python scripts/wiki_affected.py $(if $(FILES),--files $(FILES),$(BASE))

deps-check: ## 一緒に上げないと壊れる依存の組み合わせを確認(ネットワークを使わない)
	python scripts/deps.py check

deps-outdated: ## 依存の新しい版を各レジストリに問い合わせる
	python scripts/deps.py outdated

# TODO: Unity の EditMode テスト(クライアントの本体を含む)を足す
test: test-rust plugin-test wiki-lint deps-check ## プッシュ前のテスト一式

sandbox-server: db-up ## sandbox のサーバーを起動(.env を読む。DB は voltech_sandbox)
	cargo run --manifest-path sandbox/server/Cargo.toml -p server
