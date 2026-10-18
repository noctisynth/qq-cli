$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$allowedFiles = @(
    '.cargo_vcs_info.json', 'Cargo.toml', 'Cargo.toml.orig', 'Cargo.lock',
    'README.md', 'CONTRIBUTING.md', 'rust-toolchain.toml'
)
$allowedPrefixes = @('src/', 'tests/', 'scripts/', 'docs/', 'skills/')

Push-Location -LiteralPath $projectRoot
try {
    $entries = @(& cargo package --list --allow-dirty --offline --locked)
    if ($LASTEXITCODE -ne 0) {
        throw 'Cannot inspect the Cargo source package'
    }
    foreach ($entry in $entries) {
        $packagePath = $entry.Replace('\', '/')
        $allowed = $packagePath -cin $allowedFiles
        foreach ($prefix in $allowedPrefixes) {
            if ($packagePath.StartsWith($prefix, [StringComparison]::Ordinal)) {
                $allowed = $true
            }
        }
        if (-not $allowed) {
            throw "Unexpected source package entry: $packagePath"
        }
    }
    Write-Output "Source package allowlist passed ($($entries.Count) files)."
}
finally {
    Pop-Location
}
