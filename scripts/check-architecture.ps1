$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$sourceRoot = Join-Path $projectRoot 'src'
$subsystems = @('app', 'cli', 'core', 'messaging', 'napcat', 'native', 'requests', 'storage', 'transport')

$rootFiles = @(Get-ChildItem -LiteralPath $sourceRoot -File -Filter '*.rs' | ForEach-Object Name | Sort-Object)
if (($rootFiles -join ',') -ne 'lib.rs,main.rs') {
    throw 'Source root must contain only the library boundary and binary entry point.'
}
$library = Get-Content -LiteralPath (Join-Path $sourceRoot 'lib.rs') -Raw
$declared = @([regex]::Matches($library, '(?m)^pub mod ([a-z_]+);') | ForEach-Object { $_.Groups[1].Value } | Sort-Object)
if (($declared -join ',') -ne (($subsystems | Sort-Object) -join ',')) {
    throw 'Library subsystem declarations differ from the documented architecture.'
}
if ($library -match 'pub use|allow\(unsafe_code\)') {
    throw 'Do not restore root compatibility aliases or widen the unsafe boundary.'
}

foreach ($subsystem in @('core', 'native', 'storage', 'transport', 'napcat')) {
    $files = Get-ChildItem -LiteralPath (Join-Path $sourceRoot $subsystem) -Recurse -File -Filter '*.rs'
    foreach ($file in $files) {
        if ($file.Name -eq 'tests.rs' -or $file.Directory.Name -eq 'tests') { continue }
        $body = Get-Content -LiteralPath $file.FullName -Raw
        # Unit-only integration fixtures may exercise higher-level composition.
        $production = ($body -split '#\[cfg\(test\)\]', 2)[0]
        $grouped = @([regex]::Matches($production, '(?s)\buse\s+crate::\{(.*?)\};') | ForEach-Object { $_.Groups[1].Value })
        $groupedUpper = @($grouped | Where-Object { $_ -match '(?:^|,)\s*(app|cli)\b' }).Count -gt 0
        if ($production -match '\bcrate::app\b|\bcrate::cli\b' -or $groupedUpper) {
            throw "Reverse dependency into application/CLI: $subsystem/$($file.Name)"
        }
        $groupedMessaging = @($grouped | Where-Object { $_ -match '(?:^|,)\s*messaging\b' }).Count -gt 0
        if ($subsystem -eq 'transport' -and ($production -match '\bcrate::messaging\b' -or $groupedMessaging)) {
            throw "Transport depends on a messaging use case: $($file.Name)"
        }
    }
}

$allowedUnsafe = Join-Path $sourceRoot 'native/mod.rs'
foreach ($file in Get-ChildItem -LiteralPath $sourceRoot -Recurse -File -Filter '*.rs') {
    if ((Get-Content -LiteralPath $file.FullName -Raw) -match '(?s)#!?\[allow\([^\]]*\bunsafe_code\b' -and $file.FullName -ne $allowedUnsafe) {
        throw 'Unsafe allowance must remain on the audited native capability declarations only.'
    }
}
$nativeBoundary = Get-Content -LiteralPath $allowedUnsafe -Raw
if ($nativeBoundary -notmatch '(?s)#\[allow\(unsafe_code\)\][^\r\n]*\s+pub mod job;' -or
    $nativeBoundary -notmatch '(?s)#\[allow\(unsafe_code\)\][^\r\n]*\s+pub\(crate\) mod credential_lock;' -or
    [regex]::Matches($nativeBoundary, '(?s)#!?\[allow\([^\]]*\bunsafe_code\b').Count -ne 2) {
    throw 'The audited unsafe allowances must apply only to native::job and native::credential_lock.'
}
if ((Get-Content -LiteralPath (Join-Path $sourceRoot 'app/mod.rs')).Count -gt 200) {
    throw 'Application composition entry exceeds its review budget; extract a focused use case.'
}
Write-Output 'Architecture boundaries: OK'
