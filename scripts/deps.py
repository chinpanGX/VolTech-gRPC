#!/usr/bin/env python3
"""基盤の依存パッケージの確認。標準ライブラリだけで動く。

    python scripts/deps.py check       (リポジトリのルートで実行。ネットワークを使わない)
    python scripts/deps.py outdated    (各レジストリに新しい版を問い合わせる)

check で確かめること(どれも、一緒に上げないと壊れる組み合わせ):
  - Unity のパッケージ(package.json)の依存が、sandbox の packages-lock.json で同じ版に解決されていること
    (lock が古い、または別のパッケージが高い版を引き込んでいると、宣言と違う版で動く)
  - Google.Protobuf が、Google.Api.CommonProtos の依存する版と同じこと
    (違うと、Unity のエディタの中で CommonProtos がエディタ同梱の Google.Protobuf に結び付き、ErrorInfo を読めなくなる)
  - Grpc.Core.Api・Grpc.Tools(scripts/proto-tools.sh)が、Grpc.Net.Client と同じ版であること
  - protoc プラグイン(tools/)の Google.Protobuf が、Unity のパッケージと同じ版であること
  - sandbox のサーバーの Cargo.toml の外部クレートが、基盤(server/rust)と同じ版であること
  - tonic の一族(tonic・tonic-*)、prost の一族(prost・prost-types)が、それぞれ同じ版であること
エラーがあれば終了コード 1。
"""
import json
import os
import re
import sys
import tomllib
import urllib.request

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
PACKAGE_JSON = "Unity/com.chinpangx.voltech.grpc/package.json"
PACKAGE_NAME = "com.chinpangx.voltech.grpc"
LOCK_JSON = "sandbox/UnityProject/Packages/packages-lock.json"
PROTO_TOOLS = "scripts/proto-tools.sh"
PLUGIN_CSPROJS = ["tools/protoc-gen-voltech-grpc/src/ProtocGenVoltechGrpc.csproj",
                  "tools/protoc-gen-voltech-grpc/tests/ProtocGenVoltechGrpc.Tests.csproj"]
CARGO_FOUNDATION = "server/rust/Cargo.toml"
CARGO_SANDBOX = "sandbox/server/Cargo.toml"

PROTOBUF = "org.nuget.google.protobuf"
COMMON_PROTOS = "org.nuget.google.api.commonprotos"
GRPC_CLIENT = "org.nuget.grpc.net.client"
GRPC_CORE = "org.nuget.grpc.core.api"


def read(path):
    with open(os.path.join(ROOT, path), encoding="utf-8") as f:
        return f.read()


def package_deps():
    return json.loads(read(PACKAGE_JSON))["dependencies"]


def lock_deps():
    return json.loads(read(LOCK_JSON))["dependencies"]


def grpc_tools_version():
    m = re.search(r"(?m)^GRPC_TOOLS_VERSION=(\S+)", read(PROTO_TOOLS))
    return m.group(1) if m else None


def buf_version():
    m = re.search(r"(?m)^BUF_VERSION=(\S+)", read(PROTO_TOOLS))
    return m.group(1) if m else None


def csproj_refs(path):
    return dict(re.findall(r'<PackageReference Include="([^"]+)" Version="([^"]+)"', read(path)))


def cargo_deps(path):
    deps = tomllib.loads(read(path))["workspace"]["dependencies"]
    out = {}
    for name, spec in deps.items():
        if isinstance(spec, str):
            out[name] = spec
        elif "version" in spec:  # path だけのもの(基盤のクレート)は外部の依存ではない
            out[name] = spec["version"]
    return out


# --- check ---

def check():
    errors = []
    pkg = package_deps()
    lock = lock_deps()

    for name, want in pkg.items():
        got = lock.get(name, {}).get("version")
        if got != want:
            errors.append("%s: package.json は %s、%s では %s に解決されている"
                          "(sandbox/UnityProject を Unity で開いて lock を更新する。別の依存が高い版を引き込んでいないかも確かめる)"
                          % (name, want, LOCK_JSON, got))

    needs = lock.get(COMMON_PROTOS, {}).get("dependencies", {}).get(PROTOBUF)
    if needs and pkg.get(PROTOBUF) != needs:
        errors.append("%s: %s %s は %s %s を参照する。package.json の %s を %s にそろえる"
                      % (PROTOBUF, COMMON_PROTOS, pkg.get(COMMON_PROTOS), PROTOBUF, needs, PROTOBUF, needs))

    grpc = pkg.get(GRPC_CLIENT)
    if pkg.get(GRPC_CORE) != grpc:
        errors.append("%s: %s(%s)と同じ版にする(いまは %s)" % (GRPC_CORE, GRPC_CLIENT, grpc, pkg.get(GRPC_CORE)))
    tools = grpc_tools_version()
    if tools != grpc:
        errors.append("%s の GRPC_TOOLS_VERSION: %s(%s)と同じ版にする(いまは %s)" % (PROTO_TOOLS, GRPC_CLIENT, grpc, tools))

    for path in PLUGIN_CSPROJS:
        refs = csproj_refs(path)
        if "Google.Protobuf" in refs and refs["Google.Protobuf"] != pkg.get(PROTOBUF):
            errors.append("%s: Google.Protobuf を Unity のパッケージ(%s)と同じ版にする(いまは %s)"
                          % (path, pkg.get(PROTOBUF), refs["Google.Protobuf"]))

    foundation = cargo_deps(CARGO_FOUNDATION)
    sandbox = cargo_deps(CARGO_SANDBOX)
    for name, version in sandbox.items():
        if name in foundation and foundation[name] != version:
            errors.append("%s: %s を基盤(%s)と同じ版にする(基盤は %s、sandbox は %s)"
                          % (CARGO_SANDBOX, name, CARGO_FOUNDATION, foundation[name], version))

    for path, deps in ((CARGO_FOUNDATION, foundation), (CARGO_SANDBOX, sandbox)):
        for family, members in (("tonic", [n for n in deps if n == "tonic" or n.startswith("tonic-")]),
                                ("prost", [n for n in deps if n in ("prost", "prost-types")])):
            versions = {deps[n] for n in members}
            if len(versions) > 1:
                errors.append("%s: %s の一族は同じ版にする(%s)"
                              % (path, family, "、".join("%s=%s" % (n, deps[n]) for n in sorted(members))))

    for e in errors:
        print("error: " + e)
    print("依存の組み合わせ / error %d" % len(errors))
    return 1 if errors else 0


# --- outdated ---

def fetch(url):
    req = urllib.request.Request(url, headers={"User-Agent": "voltech-grpc-deps", "Accept": "application/json"})
    with urllib.request.urlopen(req, timeout=20) as f:
        return json.load(f)


def is_stable(version):
    return "-" not in version


def version_key(version):
    return [int(x) if x.isdigit() else x for x in re.split(r"[.\-+]", version)]


def upm_registry(name):
    if name.startswith("org.nuget."):
        return "https://unitynuget-registry.openupm.com"
    return "https://package.openupm.com"


def upm_latest(name):
    doc = fetch("%s/%s" % (upm_registry(name), name))
    stable = sorted((v for v in doc["versions"] if is_stable(v)), key=version_key)
    latest = stable[-1] if stable else None
    return latest, doc["versions"].get(latest, {}).get("dependencies", {})


def nuget_latest(name):
    doc = fetch("https://api.nuget.org/v3-flatcontainer/%s/index.json" % name.lower())
    stable = [v for v in doc["versions"] if is_stable(v)]
    return stable[-1] if stable else None


def crate_latest(name):
    return fetch("https://crates.io/api/v1/crates/%s" % name)["crate"]["max_stable_version"]


def cargo_compatible(requirement, version):
    """Cargo の既定の要求(^)の範囲に入るか。0.x は minor まで、1 以上は major が同じなら入る。"""
    req = [int(x) for x in requirement.split(".")]
    ver = [int(x) for x in re.split(r"[+\-]", version)[0].split(".")[:3]]  # 1.0.8+spec-1 のような付記は外す
    if req[0] != ver[0]:
        return False
    if req[0] == 0 and len(req) > 1:
        return req[1] == ver[1]
    return True


def outdated():
    rows = []  # (場所, 名前, いま, 最新, 備考)

    def row(where, name, current, latest, note=""):
        mark = "" if latest is None or current == latest else "*"
        rows.append((mark, where, name, current, latest or "?", note))

    def safe(fn, *args):
        try:
            return fn(*args)
        except Exception as e:  # 1つ取れなくても残りは出す
            print("warn: %s %s を取得できない: %s" % (fn.__name__, args[0], e))
            return None

    # Google.Protobuf は、最新の CommonProtos が参照する版までしか上げられない
    got = safe(upm_latest, COMMON_PROTOS)
    protobuf_cap = got[1].get(PROTOBUF) if got else None
    protobuf_note = "%s の最新が参照する %s までしか上げない" % (COMMON_PROTOS, protobuf_cap) if protobuf_cap else ""

    for name, current in package_deps().items():
        got = safe(upm_latest, name)
        latest, deps = got if got else (None, {})
        note = ""
        if name == COMMON_PROTOS and latest and PROTOBUF in deps:
            note = "最新は %s %s を参照する" % (PROTOBUF, deps[PROTOBUF])
        if name == PROTOBUF:
            note = protobuf_note
        if name == GRPC_CORE:
            note = "%s と同じ版にする" % GRPC_CLIENT
        row("Unity", name, current, latest, note)

    row("proto-tools", "Grpc.Tools", grpc_tools_version(), safe(nuget_latest, "Grpc.Tools"), "Grpc.Net.Client と同じ版にする")
    buf = None
    try:
        buf = fetch("https://api.github.com/repos/bufbuild/buf/releases/latest")["tag_name"].lstrip("v")
    except Exception as e:
        print("warn: buf の最新を取得できない: %s" % e)
    row("proto-tools", "buf", buf_version(), buf)

    for path in PLUGIN_CSPROJS:
        for name, current in csproj_refs(path).items():
            note = "Unity のパッケージと同じ版にする" if name == "Google.Protobuf" else ""
            row(os.path.basename(path), name, current, safe(nuget_latest, name), note)

    seen = {}
    for path in (CARGO_FOUNDATION, CARGO_SANDBOX):
        for name, requirement in cargo_deps(path).items():
            if name in seen:
                continue
            seen[name] = True
            latest = safe(crate_latest, name)
            if latest is None:
                row("Rust", name, requirement, None)
                continue
            inside = cargo_compatible(requirement, latest)
            # 要求の範囲内なら cargo update で上がるので、印を付けない
            rows.append(("" if inside else "*", "Rust", name, requirement, latest,
                         "" if inside else "要求の範囲外(Cargo.toml を書き換える)"))

    widths = [max(len(str(r[i])) for r in rows) for i in range(5)]
    for r in rows:
        print("%-1s %-*s  %-*s  %-*s -> %-*s  %s" % (r[0], widths[1], r[1], widths[2], r[2], widths[3], r[3], widths[4], r[4], r[5]))
    print("* は新しい版がある(Rust は Cargo.toml の要求の範囲外のものだけ)。上げるときは /deps-update の手順で、組み合わせごとに上げる")
    return 0


if __name__ == "__main__":
    if len(sys.argv) != 2 or sys.argv[1] not in ("check", "outdated"):
        print(__doc__)
        sys.exit(2)
    sys.exit(check() if sys.argv[1] == "check" else outdated())
