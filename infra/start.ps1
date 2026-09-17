<#
.SYNOPSIS
  Starts (or updates) Orbit's local services: SearXNG (web search) and LibreTranslate (translation).

.DESCRIPTION
  Docker runs inside WSL. The Dev Drive is not mounted there, so the config is copied to
  ~/.orbit/infra in the default WSL distro and started with docker compose from there.
  The containers restart automatically with Docker. Run again after changing the config.

.PARAMETER Update
  Pull the latest images first.
#>
param([switch]$Update)

$ErrorActionPreference = 'Stop'
$target = '~/.orbit/infra'
# WSL cannot translate a working directory on the Dev Drive.
Push-Location $env:USERPROFILE

function Invoke-Wsl([string]$command, [string]$stdin) {
  if ($PSBoundParameters.ContainsKey('stdin')) {
    $stdin | wsl.exe -- bash -c $command
  } else {
    wsl.exe -- bash -c $command
  }
  if ($LASTEXITCODE -ne 0) { throw "WSL command failed ($LASTEXITCODE): $command" }
}

function Read-Text([string]$path) {
  (Get-Content $path -Raw) -replace "`r`n", "`n"
}

function Wait-Http([string]$name, [string]$url, [int]$seconds) {
  $deadline = (Get-Date).AddSeconds($seconds)
  while ((Get-Date) -lt $deadline) {
    try {
      Invoke-RestMethod $url -TimeoutSec 5 | Out-Null
      Write-Host "  $name is ready"
      return
    } catch {
      Start-Sleep -Seconds 2
    }
  }
  throw "$name did not respond at $url. Check: wsl -- docker logs orbit-$($name.ToLower())"
}

try {
  Invoke-Wsl "mkdir -p $target/searxng"

  # Keep an existing SearXNG secret so restarts don't invalidate anything; otherwise generate one.
  $secret = "$(wsl.exe -- bash -c "grep -o '[0-9a-f]\{64\}' $target/searxng/settings.yml 2>/dev/null || true")".Trim()
  if (-not $secret) { $secret = -join ((1..32) | ForEach-Object { '{0:x2}' -f (Get-Random -Maximum 256) }) }

  Invoke-Wsl "cat > $target/searxng/settings.yml" (Read-Text "$PSScriptRoot\searxng\settings.yml").Replace('__SECRET__', $secret)
  Invoke-Wsl "cat > $target/compose.yaml" (Read-Text "$PSScriptRoot\compose.yaml")

  if ($Update) { Invoke-Wsl "cd $target && docker compose pull" }
  Invoke-Wsl "cd $target && docker compose up -d --remove-orphans"

  Write-Host 'Waiting for the services (the first start downloads the translation models, a few minutes):'
  Wait-Http 'SearXNG' 'http://127.0.0.1:8888/search?q=orbit&format=json' 60
  Wait-Http 'LibreTranslate' 'http://127.0.0.1:8890/languages' 600
  Write-Host 'Orbit services are running: search http://127.0.0.1:8888, translation http://127.0.0.1:8890'
} finally {
  Pop-Location
}
