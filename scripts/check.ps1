$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot

function Invoke-CargoCheck {
    param([string[]] $CargoArguments)
    & cargo @CargoArguments
    if ($LASTEXITCODE -ne 0) {
        throw "Cargo check failed: $($CargoArguments -join ' ')"
    }
}

Push-Location -LiteralPath $projectRoot
try {
    & (Join-Path $PSScriptRoot 'check-architecture.ps1')
    & (Join-Path $PSScriptRoot 'check-contracts.ps1')
    & (Join-Path $PSScriptRoot 'check-release.ps1')
    Invoke-CargoCheck -CargoArguments @('fmt', '--all', '--', '--check')
    Invoke-CargoCheck -CargoArguments @('clippy', '--all-targets', '--offline', '--locked', '-j1', '--', '-D', 'warnings')
    Invoke-CargoCheck -CargoArguments @('test', '--offline', '--locked', '-j1', '--', '--test-threads=1')
    Invoke-CargoCheck -CargoArguments @('build', '--release', '--offline', '--locked', '-j1')
    & (Join-Path $PSScriptRoot 'check-package.ps1')
}
finally {
    Pop-Location
}
