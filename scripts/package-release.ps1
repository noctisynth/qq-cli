param([string] $OutputDirectory = (Join-Path (Split-Path -Parent $PSScriptRoot) 'artifacts'))
$ErrorActionPreference = 'Stop'
if (-not $IsWindows) { throw 'Windows x64 release packaging requires a Windows runner.' }
$projectRoot = Split-Path -Parent $PSScriptRoot
Import-Module (Join-Path $PSScriptRoot 'release.psm1') -Force
Push-Location -LiteralPath $projectRoot
try {
    $metadata = & cargo metadata --offline --locked --no-deps --format-version 1
    if ($LASTEXITCODE -ne 0) { throw 'Cannot read locked Cargo package metadata.' }
    $metadata = $metadata | ConvertFrom-Json
    $package = @($metadata.packages | Where-Object name -EQ 'qq-cli')
    if ($package.Count -ne 1) { throw 'Expected one qq-cli package.' }
    & cargo build --release --offline --locked --target x86_64-pc-windows-msvc -j1
    if ($LASTEXITCODE -ne 0) { throw 'Windows release build failed.' }
    $executable = Join-Path $metadata.target_directory 'x86_64-pc-windows-msvc/release/qq-cli.exe'
    $version = $package[0].version
    $actualVersion = & $executable --version
    if ($LASTEXITCODE -ne 0 -or $actualVersion -notmatch "^qq-cli(?:\.exe)? $([regex]::Escape($version))$") {
        throw 'Executable version differs from Cargo package metadata.'
    }
    $result = New-ReleasePackage -Version $version -Executable $executable -Readme (Join-Path $projectRoot 'README.md') `
        -SkillRoot (Join-Path $projectRoot 'skills/qq-cli') -OutputDirectory $OutputDirectory
    Write-Output "Checked Windows x64 package v$($result.Version): $($result.Entries) entries and SHA256SUMS."
} finally { Pop-Location }
