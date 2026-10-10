#!/usr/bin/env bash
# .proto の生成に使うツール(buf・protoc・grpc_csharp_plugin)を .tools/bin に入れる。
# 入っているバージョンが同じなら何もしない。
set -euo pipefail

BUF_VERSION=1.73.0
# Unity の Packages/manifest.json の org.nuget.grpc.net.client と同じにする
GRPC_TOOLS_VERSION=2.84.0

root=$(cd "$(dirname "$0")/.." && pwd)
bin="$root/.tools/bin"
stamp="$bin/.versions"
want="buf=$BUF_VERSION grpc-tools=$GRPC_TOOLS_VERSION"

if [ -f "$stamp" ] && [ "$(cat "$stamp")" = "$want" ]; then
    exit 0
fi

case "$(uname -s)/$(uname -m)" in
    MINGW*/x86_64 | MSYS*/x86_64) buf_asset=buf-Windows-x86_64.exe; grpc_dir=windows_x64; exe=.exe ;;
    Darwin/arm64) buf_asset=buf-Darwin-arm64; grpc_dir=macosx_universal; exe= ;;
    Darwin/x86_64) buf_asset=buf-Darwin-x86_64; grpc_dir=macosx_universal; exe= ;;
    Linux/x86_64) buf_asset=buf-Linux-x86_64; grpc_dir=linux_x64; exe= ;;
    Linux/aarch64 | Linux/arm64) buf_asset=buf-Linux-aarch64; grpc_dir=linux_arm64; exe= ;;
    *)
        echo "このOS・CPUには対応していない: $(uname -s)/$(uname -m)" >&2
        exit 1
        ;;
esac

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$bin"

echo "buf $BUF_VERSION をダウンロードする"
curl -fsSL "https://github.com/bufbuild/buf/releases/download/v$BUF_VERSION/$buf_asset" -o "$bin/buf$exe"

# protoc と grpc_csharp_plugin は、NuGet の Grpc.Tools パッケージに全OS分が入っている
echo "Grpc.Tools $GRPC_TOOLS_VERSION(protoc・grpc_csharp_plugin)をダウンロードする"
curl -fsSL "https://www.nuget.org/api/v2/package/Grpc.Tools/$GRPC_TOOLS_VERSION" -o "$tmp/grpc.tools.zip"
unzip -q "$tmp/grpc.tools.zip" "tools/$grpc_dir/*" -d "$tmp"
cp "$tmp/tools/$grpc_dir/"* "$bin/"

# zip から取り出すと実行権限が外れるので付け直す
chmod +x "$bin"/*
echo "$want" >"$stamp"

"$bin/buf$exe" --version
"$bin/protoc$exe" --version
