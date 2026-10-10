#!/usr/bin/env python3
"""docs/wiki の API の表を .proto から生成する。標準ライブラリだけで動く。

    python scripts/wiki_gen.py           (Shared フォルダで実行。ページを書き換える)
    python scripts/wiki_gen.py --check   (書き換えずに、生成し直すと変わるページがあれば終了コード 1)

ページに次の目印を置くと、その間を RPC の説明・リクエスト・レスポンスの表で置き換える。

    <!-- generated:rpc voltech.v1.DeviceService/Register -->
    <!-- /generated -->

RPC は、そのページの frontmatter の proto: に書いた .proto から探す。
説明は .proto のコメント(直前の行の //)から取る。表を直すときは .proto のコメントを直す。

読めるのは、このリポジトリの .proto の書き方(1行に1つの宣言)だけ。読めない書き方があれば、黙って飛ばさずにエラーにする。
"""
import os
import re
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))  # Shared/
WIKI = os.path.join(ROOT, "docs", "wiki")
SKIP_FILES = {"CLAUDE.md"}
SKIP_DIRS = {".obsidian", ".trash"}

BLOCK = re.compile(r"(<!-- generated:rpc (\S+) -->\n)(.*?)(<!-- /generated -->)", re.S)

SCALARS = {
    "double", "float", "int32", "int64", "uint32", "uint64", "sint32", "sint64",
    "fixed32", "fixed64", "sfixed32", "sfixed64", "bool", "string", "bytes",
}
IDEMPOTENCY = {
    "NO_SIDE_EFFECTS": "`NO_SIDE_EFFECTS`(副作用なし)",
    "IDEMPOTENT": "`IDEMPOTENT`(再送してよい)",
    "IDEMPOTENCY_UNKNOWN": "`IDEMPOTENCY_UNKNOWN`",
}


class ProtoError(Exception):
    pass


def join_comment(lines):
    """コメントの行をつなぐ。日本語の行末・行頭では空白を入れない"""
    out = ""
    for line in lines:
        if out and line and ord(out[-1]) < 0x3000 and ord(line[0]) < 0x3000:
            out += " "
        out += line
    return out


def parse_proto(path):
    """package・サービス・メッセージを読む。メッセージは完全名(package.Outer.Inner)で持つ"""
    lines = open(path, encoding="utf-8-sig").read().split("\n")
    package = ""
    services, messages, enums = {}, {}, set()
    stack = []  # (種類, 名前, 持ち物)
    comment = []

    def where(i):
        return "%s:%d" % (os.path.relpath(path, ROOT).replace(os.sep, "/"), i + 1)

    def scope():
        return ".".join([package] + [n for k, n, _ in stack if k == "message"]).strip(".")

    for i, raw in enumerate(lines):
        line = raw.strip()
        if line.startswith("/*"):
            raise ProtoError("%s: /* */ のコメントは読めない" % where(i))
        if line.startswith("//"):
            comment.append(line[2:].strip())
            continue
        trailing = ""
        if "//" in line:
            line, _, trailing = line.partition("//")
            line, trailing = line.strip(), trailing.strip()
        if not line:
            comment = []  # 空行で切れたコメントは、次の宣言の説明にしない(protoc と同じ)
            continue
        doc = join_comment(comment) or trailing
        comment = []
        kind = stack[-1][0] if stack else "file"

        if line == "}" or line == "};":
            if not stack:
                raise ProtoError("%s: 対応する { がない" % where(i))
            stack.pop()
            continue
        if kind == "file":
            m = re.match(r"^package\s+([\w.]+)\s*;$", line)
            if m:
                package = m.group(1)
                continue
            if re.match(r"^(syntax|edition|import|option)\b.*;$", line):
                continue
        if kind in ("file", "message"):
            m = re.match(r"^message\s+(\w+)\s*\{$", line)
            if m:
                full = scope() + "." + m.group(1) if scope() else m.group(1)
                messages[full] = {"doc": doc, "fields": [], "where": where(i)}
                stack.append(("message", m.group(1), messages[full]))
                continue
            m = re.match(r"^enum\s+(\w+)\s*\{$", line)
            if m:
                enums.add(scope() + "." + m.group(1) if scope() else m.group(1))
                stack.append(("enum", m.group(1), None))
                continue
        if kind == "file":
            m = re.match(r"^service\s+(\w+)\s*\{$", line)
            if m:
                svc = {"doc": doc, "rpcs": {}, "skip_auth": False}
                services[package + "." + m.group(1)] = svc
                stack.append(("service", m.group(1), svc))
                continue
        if kind == "service":
            svc = stack[-1][2]
            if re.match(r"^option\s+\((auth\.v1\.)?skip_auth\)\s*=\s*true\s*;$", line):
                svc["skip_auth"] = True
                continue
            if line.startswith("option "):
                continue
            m = re.match(r"^rpc\s+(\w+)\s*\(\s*(stream\s+)?([\w.]+)\s*\)\s*returns\s*\(\s*(stream\s+)?([\w.]+)\s*\)\s*(\{|;|\{\s*\})$", line)
            if m:
                rpc = {"doc": doc, "request": m.group(3), "response": m.group(5),
                       "stream": bool(m.group(2) or m.group(4)), "idempotency": None}
                svc["rpcs"][m.group(1)] = rpc
                if m.group(6) == "{":
                    stack.append(("rpc", m.group(1), rpc))
                continue
        if kind == "rpc":
            m = re.match(r"^option\s+idempotency_level\s*=\s*(\w+)\s*;$", line)
            if m:
                stack[-1][2]["idempotency"] = m.group(1)
                continue
            if line.startswith("option "):
                continue
        if kind == "enum":
            continue  # 値は表にしない
        if kind in ("message", "oneof"):
            msg = next(p for k, _, p in reversed(stack) if k == "message")
            oneof = stack[-1][1] if kind == "oneof" else None
            m = re.match(r"^oneof\s+(\w+)\s*\{$", line)
            if m and kind == "message":
                stack.append(("oneof", m.group(1), None))
                continue
            if re.match(r"^(reserved|option)\b.*;$", line):
                continue
            m = re.match(r"^(?:(repeated|optional)\s+)?(map\s*<\s*[\w.]+\s*,\s*[\w.]+\s*>|[\w.]+)\s+(\w+)\s*=\s*\d+\s*(\[.*\])?\s*;$", line)
            if m:
                msg["fields"].append({"label": m.group(1), "type": re.sub(r"\s+", "", m.group(2)).replace(",", ", "),
                                      "name": m.group(3), "doc": doc, "oneof": oneof, "scope": scope()})
                continue
        raise ProtoError("%s: 読めない行: %s" % (where(i), raw.strip()))
    if stack:
        raise ProtoError("%s: } が足りない" % path)
    return services, messages, enums


def resolve(name, scope, messages):
    """型の名前を、protoc と同じく内側のスコープから順に探す"""
    if name.startswith("."):
        return name[1:] if name[1:] in messages else None
    parts = scope.split(".") if scope else []
    while True:
        cand = ".".join(parts + [name])
        if cand in messages:
            return cand
        if not parts:
            return None
        parts.pop()


def text(s):
    """コメントを Markdown に入れる。<...> が HTML のタグとして消えないようにする"""
    return s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;") if s else ""


def cell(s):
    return text(s).replace("|", "\\|")


def type_cell(f, messages):
    t = f["type"]
    if t.startswith("map<"):
        shown = t
    elif t in SCALARS:
        shown = t
    else:
        shown = t.split(".")[-1] if resolve(t, f["scope"], messages) else t
    if f["label"]:
        shown = f["label"] + " " + shown
    return "`%s`" % shown if t not in SCALARS else shown


def message_table(full, messages, title, out, seen):
    msg = messages[full]
    out.append("**%s** `%s`" % (title, full.split(".")[-1]))
    out.append("")
    if msg["doc"]:
        out.append(text(msg["doc"]))
        out.append("")
    if not msg["fields"]:
        out.append("フィールドなし")
        out.append("")
        return
    out.append("| フィールド | 型 | 内容 |")
    out.append("|---|---|---|")
    nested = []
    for f in msg["fields"]:
        doc = f["doc"]
        if f["oneof"]:
            doc = ("(`%s` のどれか1つ) " % f["oneof"]) + doc
        out.append("| `%s` | %s | %s |" % (f["name"], type_cell(f, messages), cell(doc)))
        t = re.sub(r"^map<[\w.]+, ", "", f["type"]).rstrip(">")
        ref = resolve(t, f["scope"], messages)
        if ref and ref not in seen:
            seen.add(ref)
            nested.append(ref)
    out.append("")
    # フィールドの型になっているメッセージ(同じ .proto のもの)も表にする
    for ref in nested:
        message_table(ref, messages, "`%s` の中身" % full.split(".")[-1], out, seen)


def render(rpc_name, protos):
    svc_name, _, method = rpc_name.partition("/")
    services, messages = {}, {}
    for p in protos:
        s, m, _ = parse_proto(p)
        services.update(s)
        messages.update(m)
    svc = services.get(svc_name)
    if svc is None or method not in svc["rpcs"]:
        raise ProtoError("%s が frontmatter の proto: のファイルに無い" % rpc_name)
    rpc = svc["rpcs"][method]
    if rpc["stream"]:
        raise ProtoError("%s: ストリーミングの RPC は表にできない" % rpc_name)
    pkg = svc_name.rsplit(".", 1)[0]
    out = ["`/%s/%s`" % (svc_name, method), ""]
    if rpc["doc"]:
        out += [text(rpc["doc"]), ""]
    out += [
        "| 認証 | 冪等性(`idempotency_level`) |",
        "|---|---|",
        "| %s | %s |" % (
            "不要(`skip_auth`)" if svc["skip_auth"] else "必要(`authorization: Bearer <access_token>`)",
            IDEMPOTENCY.get(rpc["idempotency"], "指定なし(再送しない)")),
        "",
    ]
    seen = set()
    for title, t in (("リクエスト", rpc["request"]), ("レスポンス", rpc["response"])):
        full = resolve(t, pkg, messages)
        if full is None:
            raise ProtoError("%s: %s の定義が proto: のファイルに無い" % (rpc_name, t))
        seen.add(full)
        message_table(full, messages, title, out, seen)
    return "\n".join(out).rstrip("\n") + "\n"


def frontmatter_protos(text):
    m = re.match(r"\A---\n(.*?)\n---\n", text, re.S)
    if not m:
        return []
    pm = re.search(r"^proto:\s*\[(.*)\]\s*$", m.group(1), re.M)
    if not pm:
        return []
    return [os.path.normpath(os.path.join(ROOT, x.strip().strip('"'))) for x in pm.group(1).split(",") if x.strip()]


def main():
    check = "--check" in sys.argv[1:]
    errors, stale = [], []
    for dp, ds, fs in os.walk(WIKI):
        ds[:] = [d for d in ds if d not in SKIP_DIRS]
        for f in sorted(fs):
            if not f.endswith(".md") or f in SKIP_FILES:
                continue
            path = os.path.join(dp, f)
            name = os.path.relpath(path, WIKI).replace(os.sep, "/")
            text = open(path, encoding="utf-8-sig").read()
            if "<!-- generated:" not in text:
                continue
            protos = frontmatter_protos(text)

            def repl(m):
                try:
                    return m.group(1) + "\n" + render(m.group(2), protos) + "\n" + m.group(4)
                except (ProtoError, OSError) as e:
                    errors.append("%s: %s" % (name, e))
                    return m.group(0)

            new = BLOCK.sub(repl, text)
            if text.count("<!-- generated:") != len(BLOCK.findall(text)):
                errors.append("%s: <!-- generated:rpc ... --> と <!-- /generated --> の組が合わない" % name)
            if new != text:
                stale.append(name)
                if not check:
                    with open(path, "w", encoding="utf-8", newline="\n") as w:
                        w.write(new)
    for e in errors:
        print("error:", e)
    for s in stale:
        print(("古い :" if check else "生成 :"), s)
    if check and stale:
        print("make wiki-gen で生成し直す")
    sys.exit(1 if errors or (check and stale) else 0)


if __name__ == "__main__":
    main()
