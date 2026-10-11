# 利用者ごとに変わる設定。基盤の Makefile と、アプリの Makefile(mk/codegen.mk 経由)が読む。
# 既定と違うときは、make を実行するフォルダの local.mk(コミットしない。例は基盤の local.mk.example)か、環境変数で上書きする。
-include local.mk

# 使う .NET SDK の dotnet(protoc プラグインのビルドに 10 以上が要る)。
# PATH で先に別の SDK が見つかるとき(Unity の .NET 8 など)は、パスで指定する
DOTNET ?= dotnet

ifeq ($(OS),Windows_NT)
# PowerShell から make を実行しても、レシピは Git Bash で動かす(既定の cmd.exe では動かないため)
GIT_BASH ?= C:/Program Files/Git/bin/bash.exe
SHELL := $(GIT_BASH)
# 単なる bash だと、PATH 上の WSL の bash.exe が起動することがあるので、パスで指定する
BASH := "$(GIT_BASH)"
else
BASH := bash
endif
