$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'install.ps1')
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
$root = Join-Path (Split-Path $PSScriptRoot -Parent) ('.verification/install-' + [Guid]::NewGuid().ToString('N'))
$null = [IO.Directory]::CreateDirectory($root)
$archive = Join-Path $root 'qq-cli-v1.2.3-windows-x86_64.zip'
$checksums = Join-Path $root 'SHA256SUMS'
$destination = Join-Path $root 'installed'
function Assert-Rejected {
    param([scriptblock] $Action)
    $failed = $false
    try { & $Action | Out-Null } catch { $failed = $true }
    if (-not $failed) { throw 'Expected rejection.' }
}
function New-SyntheticArchive {
    param([byte[]] $Bytes, [switch] $Duplicate)
    if ([IO.File]::Exists($archive)) { [IO.File]::Delete($archive) }
    $zip = [IO.Compression.ZipFile]::Open($archive, [IO.Compression.ZipArchiveMode]::Create)
    try {
        $names = @('qq-cli.exe', 'LICENSE-MIT', 'LICENSE-APACHE', '../outside.txt')
        if ($Duplicate) { $names += 'qq-cli.exe' }
        foreach ($name in $names) {
            $stream = $zip.CreateEntry($name).Open()
            try { $stream.Write($Bytes, 0, $Bytes.Length) } finally { $stream.Dispose() }
        }
    } finally { $zip.Dispose() }
    $hash = (Get-FileHash -LiteralPath $archive).Hash
    [IO.File]::WriteAllText($checksums, "$hash  $([IO.Path]::GetFileName($archive))`n")
}
try {
    $pe = [byte[]]::new(256)
    $pe[0] = 0x4d; $pe[1] = 0x5a; $pe[0x3c] = 128
    $pe[128] = 0x50; $pe[129] = 0x45; $pe[132] = 0x64; $pe[133] = 0x86
    New-SyntheticArchive $pe
    $target = Install-QqCliArchive $archive $checksums $destination
    if (-not [IO.File]::Exists($target) -or (Test-Path (Join-Path $root 'outside.txt'))) { throw 'Extraction scope failed.' }
    $original = (Get-FileHash $target).Hash
    $null = Install-QqCliArchive $archive $checksums $destination
    if ((Get-FileHash $target).Hash -ne $original) { throw 'Repeated install changed executable.' }
    $pe[200] = 42
    New-SyntheticArchive $pe
    $null = Install-QqCliArchive $archive $checksums $destination
    $upgraded = (Get-FileHash $target).Hash
    if ($upgraded -eq $original) { throw 'Upgrade did not replace executable.' }
    [IO.File]::WriteAllText($checksums, (('0' * 64) + "  $([IO.Path]::GetFileName($archive))`n"))
    Assert-Rejected { Install-QqCliArchive $archive $checksums $destination }
    if ((Get-FileHash $target).Hash -ne $upgraded) { throw 'Checksum failure changed installation.' }
    $pe[132] = 0x4c; $pe[133] = 1
    New-SyntheticArchive $pe
    Assert-Rejected { Install-QqCliArchive $archive $checksums $destination }
    if ((Get-FileHash $target).Hash -ne $upgraded) { throw 'Architecture failure changed installation.' }
    New-SyntheticArchive $pe -Duplicate
    Assert-Rejected { Install-QqCliArchive $archive $checksums $destination }
    if (@(Get-ChildItem $destination -Force).Count -ne 3) { throw 'Staged files leaked.' }
    $merged = Merge-QqCliPath -Existing ('other;' + $destination.ToUpperInvariant() + '\;last') -Directory $destination
    if ($merged -cne "$destination;other;last") { throw 'PATH deduplication failed.' }
    if ((Merge-QqCliPath -Existing '' -Directory $destination) -cne $destination) { throw 'Empty PATH failed.' }
    function Invoke-RestMethod { throw 'Synthetic missing release / network failure' }
    Assert-Rejected { Get-QqCliRelease }
    function Invoke-RestMethod { [pscustomobject]@{ draft = $false; prerelease = $false; assets = @() } }
    Assert-Rejected { Get-QqCliRelease }
    function Invoke-RestMethod {
        [pscustomobject]@{ draft = $false; prerelease = $false; assets = @(
            [pscustomobject]@{ name = 'qq-cli-v1.2.3-windows-x86_64.zip'; browser_download_url = 'https://github.com/noctisynth/qq-cli/releases/download/v1.2.3/qq-cli-v1.2.3-windows-x86_64.zip' },
            [pscustomobject]@{ name = 'SHA256SUMS'; browser_download_url = 'https://github.com/noctisynth/qq-cli/releases/download/v1.2.3/SHA256SUMS' }
        ) }
    }
    if ((Get-QqCliRelease).Archive.name -cne 'qq-cli-v1.2.3-windows-x86_64.zip') { throw 'Latest asset selection failed.' }
    $pe[132] = 0x64; $pe[133] = 0x86; $pe[201] = 99
    New-SyntheticArchive $pe
    $locked = [IO.File]::Open($target, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::None)
    try { Assert-Rejected { Install-QqCliArchive $archive $checksums $destination } } finally { $locked.Dispose() }
    if ((Get-FileHash $target).Hash -ne $upgraded) { throw 'Locked executable was changed.' }
    $script:fixtureArchive = $archive
    $script:fixtureChecksums = $checksums
    function Invoke-WebRequest {
        param($Uri, $OutFile, $TimeoutSec, $ErrorAction, [switch] $UseBasicParsing)
        if ($Uri.EndsWith('/SHA256SUMS')) { Copy-Item -LiteralPath $script:fixtureChecksums -Destination $OutFile }
        else { Copy-Item -LiteralPath $script:fixtureArchive -Destination $OutFile }
    }
    $originalPath = $env:Path
    $result = Install-QqCli -Directory $destination -SkipPath
    if ($result.pathUpdated -or $env:Path -cne $originalPath -or (Get-FileHash $target).Hash -eq $upgraded) { throw 'Full installation or NoPath failed.' }
    Write-Output 'Installer checks passed: install/repeat/upgrade, checksum and architecture rejection, archive isolation, staging cleanup, PATH and missing release.'
} finally {
    $resolved = [IO.Path]::GetFullPath($root)
    $boundary = [IO.Path]::GetFullPath((Join-Path (Split-Path $PSScriptRoot -Parent) '.verification')) + [IO.Path]::DirectorySeparatorChar
    if (-not $resolved.StartsWith($boundary, [StringComparison]::OrdinalIgnoreCase)) { throw 'Unsafe cleanup target.' }
    Remove-Item -LiteralPath $resolved -Recurse -Force
}
