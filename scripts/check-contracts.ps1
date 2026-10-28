$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$sourceRoot = Join-Path $projectRoot 'src'

foreach ($file in Get-ChildItem -LiteralPath $sourceRoot -Recurse -File -Filter '*.rs') {
    $body = Get-Content -LiteralPath $file.FullName -Raw
    $production = ($body -split '#\[cfg\(test\)\]', 2)[0]
    if ($production -cmatch 'Envelope::success\s*\(|Envelope::partial\s*\(|\bLegacyWire\b') {
        throw "Legacy response bridge restored: $($file.Name)"
    }
}
$contracts = Join-Path $sourceRoot 'core/contracts'
foreach ($file in Get-ChildItem -LiteralPath $contracts -Recurse -File -Filter '*.rs') {
    $body = Get-Content -LiteralPath $file.FullName -Raw
    if ($body -cmatch '(?m)^\s*pub\s+\w+\s*:[^\r\n]*\bValue\b') {
        throw "Unconstrained JSON field in public DTO: $($file.Name)"
    }
    $apiProtocol = Join-Path $contracts 'api/json.rs'
    if ($file.Name -ne 'discovery.rs' -and $file.FullName -ne $apiProtocol -and $body -cmatch '\bValue\b') {
        throw "Open protocol JSON is allowed only in SchemaDocument: $($file.Name)"
    }
}
$output = Get-Content -LiteralPath (Join-Path $sourceRoot 'core/output.rs') -Raw
if ($output -cmatch 'serde_json::|\bValue\b') { throw 'Response dispatch must remain closed and typed.' }
$schema = Get-Content -LiteralPath (Join-Path $sourceRoot 'cli/schema.rs') -Raw
if ($schema -cmatch 'json!|"properties"|"required"|"oneOf"|"anyOf"') { throw 'Output schemas must be generated from actual types.' }
$registry = Get-Content -LiteralPath (Join-Path $sourceRoot 'cli/registry.rs') -Raw
if ($registry -cmatch 'id\s*==|matches!\(\s*id\s*,') { throw 'Input schema types must follow Clap parsers, not parameter names.' }
Write-Output 'Single-source contract boundaries: OK'
