"""docs/wiki のスクリプト(wiki_lint.py・wiki_affected.py)で共通に使う、ページの探し方と frontmatter の読み方。

frontmatter の形を変えるときは、ここだけを直す(2つのスクリプトで読み方が食い違わないように)。
"""
import os
import re

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))  # リポジトリのルート
REPO = ROOT  # frontmatter の code・proto はここからのパス
WIKI = os.path.join(ROOT, "docs", "wiki")
SKIP_FILES = {"CLAUDE.md"}
SKIP_DIRS = {".obsidian", ".trash"}  # Obsidian の設定とゴミ箱


def iter_pages():
    """wiki のページ(.md)のパスを返す。CLAUDE.md と Obsidian のフォルダは除く。"""
    for dp, ds, fs in os.walk(WIKI):
        ds[:] = [d for d in ds if d not in SKIP_DIRS]
        for f in fs:
            if f.endswith(".md") and f not in SKIP_FILES:
                yield os.path.normpath(os.path.join(dp, f))


def page_name(path):
    """wiki のルートからのページの名前(api/errors.md)。"""
    return os.path.relpath(path, WIKI).replace(os.sep, "/")


def parse(path):
    """frontmatter(dict)と本文を返す。frontmatter が無ければ (None, 全文)。"""
    # utf-8-sig: Windows のエディタが付ける BOM を読み飛ばす
    text = open(path, encoding="utf-8-sig").read()
    m = re.match(r"\A---\n(.*?)\n---\n(.*)\Z", text, re.S)
    if not m:
        return None, text
    # 値は文字列のまま持つ。YAML のブロック形式のリスト(次の行から「  - x」)は list にする
    fm, key = {}, None
    for line in m.group(1).split("\n"):
        item = re.match(r"^\s+-\s*(.*)$", line)
        if item and key is not None:
            if not isinstance(fm[key], list):
                fm[key] = []
            fm[key].append(item.group(1).strip().strip('"'))
            continue
        k, _, v = line.partition(":")
        key = k.strip()
        fm[key] = v.strip()
    return fm, m.group(2)


def as_list(name, fm, key, errors=None):
    """[a, b] の形か、ブロック形式のリストを読む。

    どちらでもない値は errors に足す(検査が黙って飛ばされないように)。errors が None なら無視する。
    """
    v = fm.get(key, "")
    if isinstance(v, list):
        return v
    if not v:
        return []
    if not (v.startswith("[") and v.endswith("]")):
        if errors is not None:
            errors.append("%s: %s はリストで書く(%s)" % (name, key, v))
        return []
    return [x.strip().strip('"') for x in v[1:-1].split(",") if x.strip()]


def exists_exact(base, relpath):
    """base の下に relpath があり、大文字・小文字まで一致するか。

    Windows と macOS の os.path.exists は大文字・小文字を区別しないので、git の実際のパスと比べる
    wiki_affected.py で一致しないパスを見逃さないように、1段ずつ名前を確かめる。
    """
    current = base
    for part in relpath.replace("\\", "/").strip("/").split("/"):
        if part in ("", "."):
            continue
        if part == "..":
            current = os.path.dirname(current)
            continue
        try:
            names = os.listdir(current)
        except (FileNotFoundError, NotADirectoryError):
            return False
        if part not in names:
            return False
        current = os.path.join(current, part)
    return True
