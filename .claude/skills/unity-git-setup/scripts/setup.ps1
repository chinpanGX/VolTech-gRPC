<#
.SYNOPSIS
  Unity プロジェクトに .gitignore / .gitattributes を配置する。

.DESCRIPTION
  -ProjectPath を省略すると、リポジトリ内の Unity プロジェクト
  (ProjectSettings/ProjectVersion.txt を持つディレクトリ) をすべて検出して処理する。
  ファイルが既に存在する場合は、テンプレートのうち未記載の行だけを末尾に追記する。

.EXAMPLE
  ./setup.ps1
  ./setup.ps1 -ProjectPath sandbox/UnityProject
  ./setup.ps1 -ProjectPath sandbox/UnityProject -NoLfs
  ./setup.ps1 -DryRun
#>
param(
    [string]$ProjectPath,
    [switch]$NoLfs,
    [switch]$Force,
    [switch]$DryRun
)

$ErrorActionPreference = 'Stop'
$templateDir = Join-Path $PSScriptRoot '..\templates' | Resolve-Path
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)

function Get-RepoRoot {
    $root = git rev-parse --show-toplevel 2>$null
    if ($LASTEXITCODE -ne 0 -or -not $root) { throw 'Git リポジトリ内で実行してください。' }
    return (Resolve-Path $root).Path
}

function Find-UnityProjects([string]$root) {
    $skip = '\\(Library|Temp|Logs|obj|Build|Builds|node_modules|\.git)\\'
    Get-ChildItem -Path $root -Recurse -Filter 'ProjectVersion.txt' -File -ErrorAction SilentlyContinue |
        Where-Object { $_.Directory.Name -eq 'ProjectSettings' -and $_.FullName -notmatch $skip } |
        ForEach-Object { $_.Directory.Parent.FullName }
}

function Get-TemplateLines([string]$name, [bool]$useLfs) {
    $text = [IO.File]::ReadAllText((Join-Path $templateDir $name), $utf8NoBom) -replace "`r`n", "`n"
    if ($useLfs) {
        $text = $text -replace '(?m)^#@LFS-(BEGIN|END)\n', ''
    } else {
        $text = $text -replace '(?s)#@LFS-BEGIN\n.*?#@LFS-END\n', ''
    }
    return $text
}

function Write-GitFile([string]$path, [string]$template) {
    $name = Split-Path $path -Leaf
    if (-not (Test-Path $path) -or $Force) {
        $action = if (Test-Path $path) { '上書き' } else { '作成' }
        if (-not $DryRun) { [IO.File]::WriteAllText($path, $template, $utf8NoBom) }
        Write-Host "  [$action] $name"
        return
    }
    # 既存ファイル: パターン行 (空行・コメント以外) のうち未記載のものだけ追記
    $existing = ([IO.File]::ReadAllText($path, $utf8NoBom) -replace "`r`n", "`n")
    $have = @{}
    foreach ($l in $existing -split "`n") { $have[$l.Trim()] = $true }
    $missing = $template -split "`n" | Where-Object {
        $t = $_.Trim(); $t -and -not $t.StartsWith('#') -and -not $have.ContainsKey($t)
    }
    if (-not $missing) { Write-Host "  [変更なし] $name"; return }
    $append = "`n# --- unity-git-setup で追加 ---`n" + (($missing -join "`n") + "`n")
    if (-not $existing.EndsWith("`n")) { $append = "`n" + $append }
    if (-not $DryRun) { [IO.File]::AppendAllText($path, $append, $utf8NoBom) }
    Write-Host "  [追記] $name ($(@($missing).Count) 行)"
}

function Test-UnitySettings([string]$project) {
    $editor = Join-Path $project 'ProjectSettings\EditorSettings.asset'
    if (-not (Test-Path $editor)) { return }
    $c = Get-Content $editor -Raw
    if ($c -match 'm_SerializationMode:\s*(\d+)' -and $Matches[1] -ne '2') {
        Write-Warning "  Asset Serialization が Force Text ではありません (m_SerializationMode: $($Matches[1]))。Project Settings > Editor で Force Text にしてください。"
    }
    if ($c -match 'm_ExternalVersionControlSupport:\s*(.+)' -and $Matches[1].Trim() -notmatch 'Visible Meta Files') {
        Write-Warning "  Version Control Mode が Visible Meta Files ではありません ($($Matches[1].Trim()))。"
    }
}

# ---------------------------------------------------------------
$repoRoot = Get-RepoRoot
$projects = if ($ProjectPath) {
    $p = (Resolve-Path $ProjectPath).Path
    if (-not (Test-Path (Join-Path $p 'ProjectSettings\ProjectVersion.txt'))) {
        throw "$p は Unity プロジェクトではありません (ProjectSettings/ProjectVersion.txt が見つかりません)。"
    }
    @($p)
} else {
    @(Find-UnityProjects $repoRoot)
}
if (-not $projects) { throw 'Unity プロジェクトが見つかりませんでした。-ProjectPath で指定してください。' }

$useLfs = -not $NoLfs
if ($useLfs) {
    git lfs version *> $null
    if ($LASTEXITCODE -ne 0) {
        Write-Warning 'git-lfs が見つからないため LFS ルールを省略します (-NoLfs 相当)。'
        $useLfs = $false
    }
}

$ignoreTpl = Get-TemplateLines 'gitignore' $useLfs
$attrTpl = Get-TemplateLines 'gitattributes' $useLfs
if ($DryRun) { Write-Host '(DryRun: ファイルは変更しません)' }

foreach ($project in $projects) {
    $rel = $project.Substring($repoRoot.Length).TrimStart('\', '/')
    Write-Host "Unity プロジェクト: $rel"
    Write-GitFile (Join-Path $project '.gitignore') $ignoreTpl
    Write-GitFile (Join-Path $project '.gitattributes') $attrTpl
    Test-UnitySettings $project

    # 既に追跡済みで、新しい ignore にマッチするファイルを報告
    $tracked = git -C $project ls-files -ci --exclude-standard -- . 2>$null
    if ($tracked) {
        Write-Warning "  ignore 対象なのに追跡済みのファイルが $(@($tracked).Count) 件あります。必要なら次を実行してください:"
        Write-Host "    git -C `"$project`" rm -r --cached --quiet -- $(($tracked | Select-Object -First 5) -join ' ') ..."
    }
}

if ($useLfs -and -not $DryRun) {
    git -C $repoRoot lfs install --local *> $null
    Write-Host 'git lfs install --local を実行しました。'
}
Write-Host '完了。'
