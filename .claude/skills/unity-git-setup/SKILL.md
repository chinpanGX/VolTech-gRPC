---
name: unity-git-setup
description: Unity プロジェクトに .gitignore と .gitattributes (Git LFS 設定含む) をセットアップする。リポジトリに Unity プロジェクトを追加したとき、Library/ や Temp/ などがコミットされそうなとき、「Unity の gitignore」「gitattributes」「LFS 設定」を頼まれたときに使う。Use when adding a Unity project to a git repo or setting up Unity .gitignore / .gitattributes / Git LFS.
---

# Unity Git セットアップ

Unity プロジェクトのルート (`Assets/` と `ProjectSettings/` がある階層) に
`.gitignore` と `.gitattributes` を配置する。モノレポ (例: `server/` と `sandbox/UnityProject/` が同居)
でも、ファイルを Unity プロジェクト直下に置くことで他ディレクトリに影響しない。

## 構成

- `templates/gitignore` — Unity 用 ignore (Library, Temp, Logs, UserSettings, IDE 生成物 など)。パターンはファイル位置基準 (`/Library/`)。
- `templates/gitattributes` — 改行 (LF) 統一、Unity YAML への `merge=unityyamlmerge`、`#@LFS-BEGIN`〜`#@LFS-END` 間に Git LFS 対象拡張子。
- `scripts/setup.ps1` — テンプレートを配置するスクリプト。

## 手順

1. **対象を確認する。** `ProjectSettings/ProjectVersion.txt` を持つディレクトリが Unity プロジェクト。
   ユーザーが場所を指定していなければスクリプトに自動検出させる。

2. **LFS を使うか決める。** 既定は LFS 有効。リモートが LFS 非対応、またはユーザーが不要と言った場合は `-NoLfs`。
   判断材料が無ければ既定 (有効) で進め、最後にその旨を伝える。

3. **まず DryRun で確認してから実行する。**
   ```powershell
   powershell -ExecutionPolicy Bypass -File .claude/skills/unity-git-setup/scripts/setup.ps1 -DryRun
   powershell -ExecutionPolicy Bypass -File .claude/skills/unity-git-setup/scripts/setup.ps1 [-ProjectPath <path>] [-NoLfs]
   ```
   - 既存の `.gitignore` / `.gitattributes` は壊さず、未記載のパターン行だけ末尾に追記する。
     全置換したいとユーザーが明言した場合のみ `-Force`。
   - LFS 有効時は `git lfs install --local` も実行する (リポジトリのフックのみ変更)。

4. **結果を検証する。**
   ```powershell
   git status --short -- <project>             # Library/ Temp/ Logs/ UserSettings/ *.csproj *.sln が出ないこと
   git check-ignore -v <project>/Library        # .gitignore の該当行が表示されること
   git check-attr -a -- <project>/Assets/x.png  # LFS 有効なら filter: lfs
   ```

5. **スクリプトの警告に対応する。**
   - *追跡済みファイル*: 既にコミットされた ignore 対象がある。`git rm -r --cached` で外すのは
     リポジトリ履歴に関わるので、実行前にユーザーへ確認する。
   - *Force Text / Visible Meta Files でない*: Unity Editor の Project Settings > Editor で変更が必要。
     Editor が起動中なら unity-cli スキルで `EditorSettings.serializationMode = SerializationMode.ForceText`
     などを eval で設定できる。
   - LFS 化する拡張子のファイルが既に通常コミットされている場合は `git lfs migrate` が必要になるが、
     履歴書き換えなので勝手に実行しない。

コミットはユーザーに頼まれたときだけ行う。

## UnityYAMLMerge (任意)

`.gitattributes` の `merge=unityyamlmerge` は merge driver を設定したときだけ有効 (未設定なら通常マージ)。
有効にしたい場合は Editor のインストール先に合わせて設定する:

```powershell
$tool = "C:/Program Files/Unity/Hub/Editor/<version>/Editor/Data/Tools/UnityYAMLMerge.exe"
git config merge.unityyamlmerge.name "Unity SmartMerge"
git config merge.unityyamlmerge.driver "'$tool' merge -p %O %B %A %A"
git config merge.unityyamlmerge.recursive binary
```

## テンプレートの変更

ignore / LFS 対象を増やすときは `templates/` を編集する。LFS 対象は必ず `#@LFS-BEGIN` と
`#@LFS-END` の間に書く (`-NoLfs` 時にその範囲が除去される)。
