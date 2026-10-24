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
# Keep the capability list authoritative instead of maintaining an unrelated count.
$nativeBoundary = Get-Content -LiteralPath $allowedUnsafe -Raw
$nativeBoundary = [regex]::Replace($nativeBoundary, '(?m)//[^\r\n]*', '')
$macosCfg = 'all(target_os = "macos", target_arch = "aarch64")'
$capabilities = @(
    @{ Cfg = 'windows'; Path = 'windows/job/mod.rs'; Declaration = 'pub mod job;' },
    @{ Cfg = 'windows'; Path = 'windows/credential_lock/mod.rs'; Declaration = 'pub(crate) mod credential_lock;' },
    @{ Cfg = $macosCfg; Path = 'macos/process.rs'; Declaration = 'pub(crate) mod macos_process;' },
    @{ Cfg = $macosCfg; Path = 'macos/socket.rs'; Declaration = 'pub mod macos_socket;' },
    @{ Cfg = $macosCfg; Path = 'macos/terminate.rs'; Declaration = 'pub mod macos_terminate;' },
    @{ Cfg = $macosCfg; Path = 'macos/signing_info.rs'; Declaration = 'pub(crate) mod macos_signing_info;' },
    @{ Cfg = $macosCfg; Path = 'macos/acl.rs'; Declaration = 'pub(crate) mod macos_acl;' },
    @{ Cfg = $macosCfg; Path = 'macos/xattr.rs'; Declaration = 'pub(crate) mod macos_xattr;' },
    @{ Cfg = $macosCfg; Path = 'macos/copy.rs'; Declaration = 'pub(crate) mod macos_copy;' },
    @{ Cfg = $macosCfg; Path = 'macos/disk.rs'; Declaration = 'pub(crate) mod macos_disk;' }
)
foreach ($entry in $capabilities) {
    $pattern = '#\[cfg\(' + [regex]::Escape($entry.Cfg) + '\)\]\s+#\[path = "' +
        [regex]::Escape($entry.Path) + '"\]\s+#\[allow\(unsafe_code\)\]\s+' +
        [regex]::Escape($entry.Declaration)
    if ($nativeBoundary -notmatch $pattern -or -not (Test-Path -LiteralPath (Join-Path $sourceRoot ('native/' + $entry.Path)))) {
        throw "Native capability must use its audited platform boundary: $($entry.Path)"
    }
}
if ([regex]::Matches($nativeBoundary, '(?s)#!?\[allow\([^\]]*\bunsafe_code\b').Count -ne $capabilities.Count) {
    throw 'Unsafe allowances must match the audited native capability list exactly.'
}
foreach ($component in @('install', 'runtime')) {
    $root = Join-Path $sourceRoot ('napcat/' + $component)
    $entry = Get-Content -LiteralPath (Join-Path $root 'mod.rs') -Raw
    $pattern = '#\[cfg\(' + [regex]::Escape($macosCfg) + '\)\]\s+pub mod macos;'
    if ($entry -notmatch $pattern -or -not (Test-Path -LiteralPath (Join-Path $root 'macos/mod.rs'))) {
        throw "macOS implementation must have one platform entry: napcat/$component/macos"
    }
    if (@(Get-ChildItem -LiteralPath $root -Filter 'macos_*').Count -ne 0) {
        throw "Do not scatter macOS modules across napcat/$component."
    }
}
if ((Get-Content -LiteralPath (Join-Path $sourceRoot 'app/mod.rs')).Count -gt 200) {
    throw 'Application composition entry exceeds its review budget; extract a focused use case.'
}

Write-Output 'Architecture boundaries: OK'
