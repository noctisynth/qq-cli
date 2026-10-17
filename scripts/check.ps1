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
    Invoke-CargoCheck -CargoArguments @('fmt', '--all', '--', '--check')
    Invoke-CargoCheck -CargoArguments @('clippy', '--all-targets', '--offline', '--locked', '--', '-D', 'warnings')
    Invoke-CargoCheck -CargoArguments @('test', '--offline', '--locked')
    Invoke-CargoCheck -CargoArguments @('build', '--release', '--offline', '--locked')
    & (Join-Path $PSScriptRoot 'check-package.ps1')
}
finally {
    Pop-Location
}
