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
    $normalizedEntries = @($entries | ForEach-Object { $_.Replace('\', '/') })
    foreach ($source in Get-ChildItem -LiteralPath (Join-Path $projectRoot 'src') -Recurse -File -Filter '*.rs') {
        $sourcePath = [IO.Path]::GetRelativePath($projectRoot, $source.FullName).Replace('\', '/')
        if ($sourcePath -cnotin $normalizedEntries) {
            throw "Rust source missing from source package: $sourcePath"
        }
    }
    Write-Output "Source package allowlist passed ($($entries.Count) files)."
}
finally {
    Pop-Location
}
