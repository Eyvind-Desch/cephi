<#
.SYNOPSIS
  Starts (or updates) the local SearXNG container that Orbit uses for web search.

.DESCRIPTION
  Docker runs inside WSL. The Dev Drive is not mounted there, so the config is copied to
  ~/.orbit/searxng in the default WSL distro and started with docker compose from there.
  The container restarts automatically with Docker. Run again after changing settings.yml.

.PARAMETER Update
  Pull the latest SearXNG image first.
#>
param([switch]$Update)

$ErrorActionPreference = 'Stop'
$target = '~/.orbit/searxng'
# WSL cannot translate a working directory on the Dev Drive.
Set-Location $env:USERPROFILE

function Invoke-Wsl([string]$command, [string]$stdin) {
  if ($PSBoundParameters.ContainsKey('stdin')) {
    $stdin | wsl.exe -- bash -c $command
  } else {
    wsl.exe -- bash -c $command
  }
  if ($LASTEXITCODE -ne 0) { throw "WSL command failed ($LASTEXITCODE): $command" }
}

Invoke-Wsl "mkdir -p $target"

# Keep an existing secret so restarts don't invalidate anything; otherwise generate one.
$secret = "$(wsl.exe -- bash -c "grep -o '[0-9a-f]\{64\}' $target/settings.yml 2>/dev/null || true")".Trim()
if (-not $secret) { $secret = -join ((1..32) | ForEach-Object { '{0:x2}' -f (Get-Random -Maximum 256) }) }

$settings = (Get-Content "$PSScriptRoot\settings.yml" -Raw).Replace('__SECRET__', $secret) -replace "`r`n", "`n"
$compose = (Get-Content "$PSScriptRoot\compose.yaml" -Raw) -replace "`r`n", "`n"
Invoke-Wsl "cat > $target/settings.yml" $settings
Invoke-Wsl "cat > $target/compose.yaml" $compose

if ($Update) { Invoke-Wsl "cd $target && docker compose pull" }
Invoke-Wsl "cd $target && docker compose up -d --force-recreate"

# Wait until the JSON API answers.
$url = 'http://127.0.0.1:8888/search?q=orbit&format=json'
foreach ($attempt in 1..30) {
  try {
    $response = Invoke-RestMethod $url -TimeoutSec 5
    Write-Host "SearXNG is running: http://127.0.0.1:8888 ($($response.results.Count) results for a test query)"
    exit 0
  } catch {
    Start-Sleep -Seconds 1
  }
}
throw "SearXNG did not respond at $url. Check: wsl -- docker logs orbit-searxng"
