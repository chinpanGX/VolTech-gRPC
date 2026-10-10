#!/usr/bin/env python3
"""docs/wiki の簡易 lint。標準ライブラリだけで動く。

    python scripts/wiki_lint.py          (リポジトリのルートで実行)

確かめること:
  - 全ページに frontmatter があり、title / status / area があること
  - status が決まった値のどれかであること
  - 相対リンクの先が存在すること(コードブロックの中は見ない)
  - frontmatter の related / proto / code が実在するページ・ファイルを指すこと(code は大文字・小文字まで)
  - 原本の節番号(sections)が複数のページに重複していないこと(警告。1つの節を複数のページに分けることがあるため)
  - どのページも index.md から辿れること(警告)
エラーがあれば終了コード 1。
"""
import os
import re
import sys
from collections import deque
from urllib.parse import unquote

from wiki_common import REPO, ROOT, WIKI, as_list as _as_list, exists_exact, iter_pages, page_name, parse

STATUSES = {"confirmed", "deferred", "current", "appendix", "open", "reference", "index"}

errors, warnings = [], []


def as_list(name, fm, key):
    return _as_list(name, fm, key, errors)


def strip_code(body):
    body = re.sub(r"^```.*?^```", "", body, flags=re.S | re.M)
    return re.sub(r"`[^`\n]*`", "", body)


# [x](a.md)・[x](a.md "タイトル")・[x](<空白 入り.md>)
LINK = re.compile(r'\]\(\s*(?:<([^>]+)>|([^)\s]+))(?:\s+"[^"]*")?\s*\)')

pages = dict.fromkeys(iter_pages())

section_owner = {}
links_of = {}
for p in sorted(pages):
    name = page_name(p)
    fm, body = parse(p)
    if fm is None:
        errors.append("%s: frontmatter がない" % name)
        continue
    for k in ("title", "status", "area"):
        if not fm.get(k):
            errors.append("%s: frontmatter に %s がない" % (name, k))
    if fm.get("status") and fm["status"] not in STATUSES:
        errors.append("%s: status '%s' は未定義(%s)" % (name, fm["status"], ", ".join(sorted(STATUSES))))
    for r in as_list(name, fm, "related"):
        if not os.path.exists(os.path.join(WIKI, r + ".md")):
            errors.append("%s: related '%s' のページがない" % (name, r))
    for pr in as_list(name, fm, "proto"):
        if not os.path.exists(os.path.join(ROOT, pr)):
            errors.append("%s: proto '%s' が存在しない" % (name, pr))
    for c in as_list(name, fm, "code"):
        # 大文字・小文字まで確かめる(git のパスと文字列で比べる wiki_affected.py で一致しなくなるため)
        if not exists_exact(REPO, c):
            errors.append("%s: code '%s' が存在しない(大文字・小文字も確かめる)" % (name, c))
    for s in as_list(name, fm, "sections"):
        if "." in s:
            if s in section_owner:
                warnings.append("%s: sections '%s' が %s と重複" % (name, s, section_owner[s]))
            section_owner[s] = name
    outs = set()
    for m in LINK.finditer(strip_code(body)):
        target = m.group(1) or m.group(2)
        if re.match(r"^(https?:|mailto:|#)", target):
            continue
        # パスの %20 などは戻してから確かめる
        t = unquote(target.split("#")[0])
        if not t:
            continue
        full = os.path.normpath(os.path.join(os.path.dirname(p), t))
        if not os.path.exists(full):
            errors.append("%s: リンク切れ %s" % (name, target))
        elif full in pages:
            outs.add(full)
    links_of[p] = outs

# index.md から辿れるか
start = os.path.normpath(os.path.join(WIKI, "index.md"))
seen, q = {start}, deque([start])
while q:
    cur = q.popleft()
    for n in links_of.get(cur, ()):
        if n not in seen:
            seen.add(n)
            q.append(n)
for p in sorted(pages):
    if p not in seen:
        warnings.append("%s: index.md から辿れない" % page_name(p))

for w in warnings:
    print("warn :", w)
for e in errors:
    print("error:", e)
print("%d ページ / error %d / warn %d" % (len(pages), len(errors), len(warnings)))
sys.exit(1 if errors else 0)
