# Windows twin of checkout-release.sh: point this clone at a release instead of main. Install and update
# both run it.
#   scripts\checkout-release.ps1               # the latest release (highest vX.Y.Z; prereleases skipped)
#   scripts\checkout-release.ps1 v0.3.0-beta   # a specific tag, prereleases included
# bin\redline.ps1 then rebuilds (cargo) or downloads that release's binary on its next run.
# Windows PowerShell 5.1 compatible. Messages go to stderr, like the .sh.
$Tag = if ($args.Count -gt 0) { [string]$args[0] } else { '' }

function Die([string]$msg) { [Console]::Error.WriteLine("error: $msg"); exit 1 }
function Say([string]$msg) { [Console]::Error.WriteLine($msg) }

if ($Tag -eq '-h' -or $Tag -eq '--help') {
  Get-Content $PSCommandPath | Select-Object -Skip 2 -First 3 | ForEach-Object { $_ -replace '^# ?', '' }
  exit 0
}

Set-Location (Split-Path -Parent $PSScriptRoot)
$remote = if ($env:REMOTE) { $env:REMOTE } else { 'origin' }

& git fetch --tags --force --quiet $remote
if ($LASTEXITCODE -ne 0) { Die "could not fetch tags from $remote" }

if (-not $Tag) {
  $Tag = & git tag --list 'v*' --sort=-v:refname | Where-Object { $_ -match '^v\d+\.\d+\.\d+$' } | Select-Object -First 1
  if (-not $Tag) { Die 'no release tags (vX.Y.Z) found' }
}
& git rev-parse -q --verify "refs/tags/$Tag" | Out-Null
if ($LASTEXITCODE -ne 0) { Die "no such tag: $Tag" }

# Never discard local edits to tracked files (untracked ones, like target\, are fine).
& git diff --quiet HEAD --
if ($LASTEXITCODE -ne 0) { Die 'this clone has local changes; commit or stash them first' }

$current = & git describe --tags --exact-match 2>$null
if ($current -eq $Tag) {
  Say "redline: already on $Tag"
} else {
  if (-not $current) { $current = & git rev-parse --short HEAD }
  & git -c advice.detachedHead=false checkout --quiet --detach $Tag
  if ($LASTEXITCODE -ne 0) { Die "could not check out $Tag" }
  Say "redline: $current -> $Tag"
}
