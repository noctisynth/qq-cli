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
foreach ($entry in @(
    @{ Path = 'windows/job/mod.rs'; Declaration = 'pub mod job;' },
    @{ Path = 'windows/credential_lock/mod.rs'; Declaration = 'pub(crate) mod credential_lock;' }
)) {
    $pattern = '#\[cfg\(windows\)\]\s+#\[path = "' + [regex]::Escape($entry.Path) + '"\]\s+#\[allow\(unsafe_code\)\][^\r\n]*\s+' + [regex]::Escape($entry.Declaration)
    if ($nativeBoundary -notmatch $pattern -or -not (Test-Path -LiteralPath (Join-Path $sourceRoot ('native/' + $entry.Path)))) {
        throw 'Windows native capabilities must select their platform-local implementation explicitly.'
    }
}
if ($nativeBoundary -notmatch '(?s)#\[allow\(unsafe_code\)\][^\r\n]*\s+pub mod job;' -or
    $nativeBoundary -notmatch '(?s)#\[allow\(unsafe_code\)\][^\r\n]*\s+pub\(crate\) mod credential_lock;' -or
    [regex]::Matches($nativeBoundary, '(?s)#!?\[allow\([^\]]*\bunsafe_code\b').Count -ne 9) {
    throw 'Unsafe allowances must remain on the nine audited native capability declarations.'
}
# Comments may wrap under rustfmt; strip them before matching declarations.
$nativeBoundary = [regex]::Replace($nativeBoundary, '(?m)//[^\r\n]*', '')
$macosPattern = '#\[cfg\(all\(target_os = "macos", target_arch = "aarch64"\)\)\]\s+#\[path = "macos/process.rs"\]\s+#\[allow\(unsafe_code\)\][^\r\n]*\s+pub\(crate\) mod macos_process;'
if ($nativeBoundary -notmatch $macosPattern -or -not (Test-Path -LiteralPath (Join-Path $sourceRoot 'native/macos/process.rs'))) {
    throw 'macOS process observations must select the audited arm64 implementation explicitly.'
}
if ((Get-Content -LiteralPath (Join-Path $sourceRoot 'app/mod.rs')).Count -gt 200) {
    throw 'Application composition entry exceeds its review budget; extract a focused use case.'
}

$socketPattern = '#\[cfg\(all\(target_os = "macos", target_arch = "aarch64"\)\)\]\s+#\[path = "macos/socket.rs"\]\s+#\[allow\(unsafe_code\)\]\s+pub mod macos_socket;'
if ($nativeBoundary -notmatch $socketPattern) { throw 'macOS socket queries must use the audited arm64 boundary.' }

$terminatePattern = '#\[cfg\(all\(target_os = "macos", target_arch = "aarch64"\)\)\]\s+#\[path = "macos/terminate.rs"\]\s+#\[allow\(unsafe_code\)\]\s+pub mod macos_terminate;'
if ($nativeBoundary -notmatch $terminatePattern) { throw 'macOS termination must use the audited generation-bound boundary.' }

$aclPattern = '#\[cfg\(all\(target_os = "macos", target_arch = "aarch64"\)\)\]\s+#\[path = "macos/acl.rs"\]\s+#\[allow\(unsafe_code\)\]\s+pub\(crate\) mod macos_acl;'
if ($nativeBoundary -notmatch $aclPattern) { throw 'macOS ACL operations must use the held-descriptor boundary.' }

$xattrPattern = '#\[cfg\(all\(target_os = "macos", target_arch = "aarch64"\)\)\]\s+#\[path = "macos/xattr.rs"\]\s+#\[allow\(unsafe_code\)\]\s+pub\(crate\) mod macos_xattr;'
if ($nativeBoundary -notmatch $xattrPattern) { throw 'macOS compression metadata must use the read-only descriptor boundary.' }

$copyPattern = '#\[cfg\(all\(target_os = "macos", target_arch = "aarch64"\)\)\]\s+#\[path = "macos/copy.rs"\]\s+#\[allow\(unsafe_code\)\]\s+pub\(crate\) mod macos_copy;'
if ($nativeBoundary -notmatch $copyPattern) { throw 'macOS copy operations must use the held-descriptor boundary.' }

$signingInfoPattern = '#\[cfg\(all\(target_os = "macos", target_arch = "aarch64"\)\)\]\s+#\[path = "macos/signing_info.rs"\]\s+#\[allow\(unsafe_code\)\]\s+pub\(crate\) mod macos_signing_info;'
if ($nativeBoundary -notmatch $signingInfoPattern) { throw 'macOS signing metadata must use the read-only CF ownership boundary.' }

Write-Output 'Architecture boundaries: OK'
