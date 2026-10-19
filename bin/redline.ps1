# Entry point on Windows (bin/redline is the Unix one; bin/redline.cmd and bin/redline run this there).
# Keeps target\bin\redline.exe current, then runs it. Agents' MCP configs and the CLI shims on PATH run
# that exe directly: Windows locks a running exe, so cargo could never rebuild one it launched itself.
# - With cargo: builds the release binary on first use, or when the sources changed.
# - Without cargo (or REDLINE_PREBUILT=1): downloads this version's release binary once.
# Windows PowerShell 5.1 compatible (every Windows has it). Errors are checked explicitly: under a "Stop"
# preference, 5.1 turns any native program's stderr output into a terminating error.
trap { [Console]::Error.WriteLine("redline: $_"); exit 1 }
$repo = Split-Path -Parent $PSScriptRoot
$env:REDLINE_REPO = $repo # the binary links this clone, not wherever it was built
$bin = Join-Path $repo 'target\bin'
$exe = Join-Path $bin 'redline.exe'

# Puts $src at $exe. The running copy can't be overwritten, but it can be renamed: move it aside and
# delete the leftovers once nothing runs them.
function Install-Exe([string]$src) {
  New-Item -ItemType Directory -Force $bin | Out-Null
  Get-ChildItem $bin -Filter 'redline.exe.old-*' -ErrorAction SilentlyContinue |
    Remove-Item -Force -ErrorAction SilentlyContinue
  if (Test-Path $exe) { Move-Item $exe "$exe.old-$([DateTime]::UtcNow.Ticks)" -Force -ErrorAction Stop }
  Copy-Item $src $exe -Force -ErrorAction Stop
  # The daemon handshake replaces a daemon started before its client's exe changed: date the copy now.
  (Get-Item $exe).LastWriteTimeUtc = [DateTime]::UtcNow
}

function Newer-Than([string]$path, [string[]]$sources) {
  $t = (Get-Item $path).LastWriteTimeUtc
  [bool](Get-ChildItem $sources -Recurse -File -ErrorAction SilentlyContinue |
    Where-Object { $_.LastWriteTimeUtc -gt $t } | Select-Object -First 1)
}

# Progress and errors go to stderr: stdout is the binary's (JSON, or MCP when an agent runs us).
function Build {
  & cargo build --release --quiet --manifest-path (Join-Path $repo 'Cargo.toml') | ForEach-Object { [Console]::Error.WriteLine($_) }
  if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
}

if ($env:REDLINE_PREBUILT -ne '1' -and (Get-Command cargo -ErrorAction SilentlyContinue)) {
  $built = Join-Path $repo 'target\release\redline.exe'
  $sources = 'src', 'web', 'migrations', 'Cargo.toml', 'build.rs' | ForEach-Object { Join-Path $repo $_ }
  if (-not (Test-Path $built)) {
    Build
  } elseif (Newer-Than $built $sources) {
    if ($args.Count -gt 0 -and $args[0] -eq 'mcp' -and (Test-Path $exe)) {
      # Agents give an MCP server ~30s to start; a rebuild takes longer. Serve this session with the
      # current binary and rebuild in the background. The next run installs it; the daemon handshake
      # picks it up from there.
      $logdir = if ($env:REDLINE_HOME) { $env:REDLINE_HOME } else { Join-Path $env:LOCALAPPDATA 'redline' }
      New-Item -ItemType Directory -Force $logdir | Out-Null
      # A build still running from an earlier session holds the log open; it'll finish this one too.
      try {
        Start-Process cargo -WindowStyle Hidden -ErrorAction Stop `
          -RedirectStandardError (Join-Path $logdir 'build.log') -RedirectStandardOutput (Join-Path $logdir 'build.out.log') `
          -ArgumentList 'build', '--release', '--quiet', '--manifest-path', "`"$(Join-Path $repo 'Cargo.toml')`""
      } catch {}
    } else {
      Build
    }
  }
  if ((Test-Path $built) -and (-not (Test-Path $exe) -or (Get-Item $built).LastWriteTimeUtc -gt (Get-Item $exe).LastWriteTimeUtc)) {
    Install-Exe $built
  }
} else {
  $version = (Select-String -Path (Join-Path $repo 'Cargo.toml') -Pattern '^version = "(.*)"' | Select-Object -First 1).Matches[0].Groups[1].Value
  $have = if (Test-Path $exe) { ((& $exe --version 2>$null) -split ' ')[1] } else { '' }
  if ($have -ne $version) {
    $arch = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
    $triple = switch ($arch) {
      'AMD64' { 'x86_64-pc-windows-msvc' }
      'ARM64' { 'aarch64-pc-windows-msvc' }
      default { [Console]::Error.WriteLine("redline: no release binary for Windows $arch; install Rust (https://rustup.rs)"); exit 1 }
    }
    $slug = if ($env:REDLINE_RELEASE_REPO) { $env:REDLINE_RELEASE_REPO } else { 'Rfluid/redline' }
    $name = "redline-$version-$triple.zip"
    $tmp = Join-Path ([IO.Path]::GetTempPath()) "redline-$([Guid]::NewGuid())"
    New-Item -ItemType Directory $tmp -ErrorAction Stop | Out-Null
    try {
      [Console]::Error.WriteLine("redline: downloading $name (no cargo found)")
      $gh = Get-Command gh -ErrorAction SilentlyContinue
      if ($gh) { & gh auth status *> $null }
      if ($gh -and $LASTEXITCODE -eq 0) {
        & gh release download "v$version" -R $slug -p $name -p "$name.sha256" -D $tmp |
          ForEach-Object { [Console]::Error.WriteLine($_) }
        if ($LASTEXITCODE -ne 0) { throw "gh release download failed" }
      } else {
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        $base = "https://github.com/$slug/releases/download/v$version"
        try {
          $ProgressPreference = 'SilentlyContinue'
          Invoke-WebRequest -UseBasicParsing "$base/$name" -OutFile (Join-Path $tmp $name) -ErrorAction Stop
          Invoke-WebRequest -UseBasicParsing "$base/$name.sha256" -OutFile (Join-Path $tmp "$name.sha256") -ErrorAction Stop
        } catch {
          [Console]::Error.WriteLine("redline: download failed; for a private repo, log in with ``gh auth login``, or install Rust")
          exit 1
        }
      }
      $want = ((Get-Content (Join-Path $tmp "$name.sha256") -Raw).Trim() -split '\s+')[0]
      $got = (Get-FileHash (Join-Path $tmp $name) -Algorithm SHA256).Hash
      if ($got -ne $want) { [Console]::Error.WriteLine("redline: checksum mismatch for $name"); exit 1 }
      Expand-Archive (Join-Path $tmp $name) -DestinationPath $tmp -ErrorAction Stop
      Install-Exe (Join-Path $tmp "redline-$version-$triple\redline.exe")
    } finally {
      Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
    }
  }
}

# Windows PowerShell 5.1 (and 7.0-7.2) pass native arguments without escaping embedded quotes, and drop
# empty ones; 7.3+ ($PSNativeCommandArgumentPassing) gets them right on its own.
function Protect-Arg([string]$a) {
  if ($PSNativeCommandArgumentPassing -and $PSNativeCommandArgumentPassing -ne 'Legacy') { return $a }
  if ($a -eq '') { return '""' }
  $a = $a -replace '(\\*)"', '$1$1\"'
  if ($a -match '\s') { $a = $a -replace '(\\+)$', '$1$1' }
  $a
}

& $exe @($args | ForEach-Object { Protect-Arg $_ })
exit $LASTEXITCODE
