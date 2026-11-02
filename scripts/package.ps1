# Builds the release binary and stages it with the docs into dist/.
#
#   pwsh -File scripts/package.ps1              # build, read the version from Cargo.toml
#   pwsh -File scripts/package.ps1 -SkipBuild   # re-zip without rebuilding
#
# Produces dist/clatuimpool-v<version>-win64.zip, ready to attach to a GitHub release.

[CmdletBinding()]
param(
    [string]$Version,
    [switch]$SkipBuild
)

$ErrorActionPreference = 'Continue'
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root

try {
    if (-not $Version) {
        $manifest = Get-Content (Join-Path $root 'Cargo.toml') -Raw
        $match = [regex]::Match($manifest, '(?m)^version\s*=\s*"([^"]+)"')
        if (-not $match.Success) { throw 'could not read the workspace version from Cargo.toml' }
        $Version = $match.Groups[1].Value
    }

    if (-not $SkipBuild) {
        cargo build --release
        if ($LASTEXITCODE -ne 0) { throw 'cargo build --release failed' }
    }

    $exe = Join-Path $root 'target\release\clatuimpool.exe'
    if (-not (Test-Path $exe)) { throw "missing release binary at $exe" }

    $name = "clatuimpool-v$Version-win64"
    $stage = Join-Path $root "dist\$name"
    if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
    New-Item -ItemType Directory -Force -Path $stage | Out-Null

    Copy-Item $exe $stage
    Copy-Item (Join-Path $root 'README.md') $stage
    Copy-Item (Join-Path $root 'LICENSE-MIT') $stage
    Copy-Item (Join-Path $root 'LICENSE-APACHE') $stage

    $zip = Join-Path $root "dist\$name.zip"
    if (Test-Path $zip) { Remove-Item $zip -Force }
    Compress-Archive -Path (Join-Path $stage '*') -DestinationPath $zip -CompressionLevel Optimal

    $size = [math]::Round((Get-Item $zip).Length / 1MB, 2)
    Write-Host "packaged dist\$name.zip ($size MiB)"
} finally {
    Pop-Location
}
