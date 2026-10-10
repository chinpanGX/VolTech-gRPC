#!/usr/bin/env python3
"""変わったファイルから、直すべき docs/wiki のページを探す。標準ライブラリだけで動く。

    python scripts/wiki_affected.py              (Shared フォルダで実行。コミットしていない変更を見る)
    python scripts/wiki_affected.py main         (main から分かれた後のこのブランチの変更と、コミットしていない変更を見る)
    python scripts/wiki_affected.py --files A B  (ファイルを直接渡す。リポジトリのルートか今のフォルダからのパス、または絶対パス)

ページの frontmatter の code(リポジトリのルートからのパス)と proto(Shared からのパス)を見て、
変わったファイルがそのパスか、その下にあれば、そのページを挙げる。
移動・名前変更したファイルは、元のパスと新しいパスの両方で探す(元の場所を説明しているページも挙がるように)。
どのページにも結びついていない変更は「ページの無い変更」として挙げる(新しいページが要るかを決める材料)。
wiki 自体の変更と、生成したコード・ロックファイルなど、説明の対象にならないものは除く。
"""
import os
import re
import subprocess
import sys

from wiki_common import REPO, as_list, iter_pages, page_name, parse

# 説明の対象にならない変更
IGNORE = [
    re.compile(r"^Shared/docs/"),
    re.compile(r"\.meta$"),
    re.compile(r"/Generated/"),
    re.compile(r"(^|/)Cargo\.lock$"),
    re.compile(r"^Unity/UnityProject/(Packages|ProjectSettings|UserSettings)/"),
    re.compile(r"^Unity/UnityProject/Assets/(Scenes|Settings)/"),
    re.compile(r"\.(unity|prefab|asset|png|jpg)$"),
]


class GitError(Exception):
    pass


def git(*args):
    # core.quotepath=off: 日本語などのパスを "\343\201..." のように引用・エスケープさせない
    result = subprocess.run(
        ["git", "-c", "core.quotepath=off", *args],
        cwd=REPO, capture_output=True, text=True, encoding="utf-8",
    )
    if result.returncode != 0:
        raise GitError("git %s が失敗した:\n%s" % (" ".join(args), result.stderr.strip()))
    return [line.strip() for line in result.stdout.splitlines() if line.strip()]


def changed_files(base):
    # --no-renames: 移動・名前変更を「元のパスの削除」と「新しいパスの追加」に分け、両方を挙げる
    diff = ["diff", "--name-only", "--no-renames"]
    if base:
        # base の先端ではなく分かれた所と比べる(分かれた後に base で変わったファイルを挙げない)。
        # 作業ツリーと比べるので、コミットしていない変更も入る
        fork_point = git("merge-base", base, "HEAD")[0]
        files = set(git(*diff, fork_point))
    else:
        files = set(git(*diff, "HEAD"))
    files |= set(git("ls-files", "--others", "--exclude-standard"))
    return sorted(files)


def normalize(path):
    """--files のパスを、リポジトリのルートからの / 区切りのパスにする。"""
    full = path if os.path.isabs(path) else None
    if full is None:
        # 今のフォルダ(make から呼ぶと Shared)からのパスとして実在すれば、そちらを使う
        candidate = os.path.abspath(path)
        full = candidate if os.path.exists(candidate) else os.path.join(REPO, path)
    rel = os.path.relpath(os.path.normpath(full), REPO)
    return rel.replace(os.sep, "/")


def load_pages():
    pages = {}
    for path in iter_pages():
        fm, _ = parse(path)
        if fm is None:
            continue
        name = page_name(path)
        targets = as_list(name, fm, "code")
        # proto は Shared からのパスなので、リポジトリのルートからのパスに直す
        for pr in as_list(name, fm, "proto"):
            targets.append(os.path.normpath(os.path.join("Shared", pr)).replace(os.sep, "/"))
        if targets:
            pages[name] = [t.rstrip("/") for t in targets]
    return pages


def covers(target, file):
    return file == target or file.startswith(target + "/")


def main():
    # Windows でパイプにつなぐと cp932 になり、日本語が化けるので UTF-8 に固定する
    sys.stdout.reconfigure(encoding="utf-8")
    sys.stderr.reconfigure(encoding="utf-8")
    args = sys.argv[1:]
    try:
        if args[:1] == ["--files"]:
            files = sorted(set(normalize(a) for a in args[1:]))
        else:
            files = changed_files(args[0] if args else None)
    except GitError as error:
        print(error, file=sys.stderr)
        sys.exit(1)
    files = [f for f in files if not any(p.search(f) for p in IGNORE)]

    pages = load_pages()
    hits, uncovered = {}, []
    for f in files:
        matched = [name for name, targets in pages.items() if any(covers(t, f) for t in targets)]
        for name in matched:
            hits.setdefault(name, []).append(f)
        if not matched:
            uncovered.append(f)

    if not files:
        print("説明の対象になる変更はない")
        return
    print("## 直すかを確かめるページ")
    for name in sorted(hits):
        print("- %s" % name)
        for f in hits[name]:
            print("    - %s" % f)
    if not hits:
        print("(なし)")
    print()
    print("## ページの無い変更")
    for f in uncovered:
        print("- %s" % f)
    if not uncovered:
        print("(なし)")


if __name__ == "__main__":
    main()
