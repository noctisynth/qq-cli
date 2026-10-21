#requires -Version 5.1
[CmdletBinding()]
param(
    [string] $InstallDirectory = (Join-Path $env:LOCALAPPDATA 'Programs\qq-cli'),
    [switch] $NoPath
)

function Get-QqCliRelease {
    $release = Invoke-RestMethod -Uri 'https://api.github.com/repos/noctisynth/qq-cli/releases/latest' -Headers @{ 'User-Agent' = 'qq-cli-installer'; Accept = 'application/vnd.github+json' } -TimeoutSec 60 -ErrorAction Stop
    if ($release.draft -or $release.prerelease) { throw 'Expected a published stable release.' }
    $archives = @($release.assets | Where-Object { $_.name -cmatch '^qq-cli-v\d+\.\d+\.\d+-windows-x86_64\.zip$' })
    $checksums = @($release.assets | Where-Object { $_.name -ceq 'SHA256SUMS' })
    if ($archives.Count -ne 1 -or $checksums.Count -ne 1) { throw 'Latest release is missing unique Windows x64 assets or SHA256SUMS.' }
    foreach ($asset in @($archives[0], $checksums[0])) {
        $uri = [Uri] $asset.browser_download_url
        if ($uri.Scheme -cne 'https' -or $uri.Host -cne 'github.com' -or
            -not $uri.AbsolutePath.StartsWith('/noctisynth/qq-cli/releases/download/', [StringComparison]::Ordinal)) {
            throw 'Unexpected release asset origin.'
        }
    }
    [pscustomobject]@{ Archive = $archives[0]; Checksums = $checksums[0] }
}

function Merge-QqCliPath {
    param([AllowEmptyString()][string] $Existing, [string] $Directory)
    $entries = @($Existing.Split(';') | Where-Object { $_.Trim().Length -gt 0 })
    $others = @($entries | Where-Object {
        [Environment]::ExpandEnvironmentVariables($_.Trim().Trim('"')).TrimEnd('\') -ine $Directory.TrimEnd('\')
    })
    (@($Directory) + $others) -join ';'
}

function Assert-QqCliInstallPath {
    param([string] $Path)
    $current = [IO.Path]::GetFullPath($Path)
    while ($current) {
        if (Test-Path -LiteralPath $current) {
            $item = Get-Item -LiteralPath $current -Force -ErrorAction Stop
            if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Installation path must not contain symbolic links or junctions.' }
        }
        $current = [IO.Path]::GetDirectoryName($current)
    }
}

function Install-QqCliArchive {
    param([string] $Archive, [string] $Checksums, [string] $Directory)
    $ErrorActionPreference = 'Stop'
    $name = [IO.Path]::GetFileName($Archive)
    $lines = @([IO.File]::ReadAllLines($Checksums) | Where-Object { $_ -match ('^[a-fA-F0-9]{64}\s+\*?' + [regex]::Escape($name) + '$') })
    if ($lines.Count -ne 1) { throw 'Missing or duplicate archive checksum.' }
    $expected = ($lines[0] -split '\s+')[0]
    if ((Get-FileHash -LiteralPath $Archive -Algorithm SHA256).Hash -ine $expected) { throw 'SHA256 verification failed; existing installation was not changed.' }
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::OpenRead($Archive)
    $stage = $null
    try {
        $executables = @($zip.Entries | Where-Object { $_.FullName -ceq 'qq-cli.exe' })
        if ($executables.Count -ne 1 -or $executables[0].Length -lt 64 -or $executables[0].Length -gt 256MB) {
            throw 'Release must contain one bounded qq-cli.exe.'
        }
        # Extract only the exact executable entry, never archive-provided paths.
        $Directory = [IO.Path]::GetFullPath($Directory)
        $target = Join-Path $Directory 'qq-cli.exe'
        Assert-QqCliInstallPath $target
        $null = [IO.Directory]::CreateDirectory($Directory)
        $stage = Join-Path $Directory ('.qq-cli-' + [Guid]::NewGuid().ToString('N') + '.tmp')
        $sourceStream = $executables[0].Open()
        try {
            $destinationStream = [IO.File]::Open($stage, [IO.FileMode]::CreateNew)
            try { $sourceStream.CopyTo($destinationStream) } finally { $destinationStream.Dispose() }
        } finally { $sourceStream.Dispose() }
        $pe = [IO.File]::ReadAllBytes($stage)
        if ($pe.Length -lt 64 -or $pe[0] -ne 0x4d -or $pe[1] -ne 0x5a) { throw 'Invalid Windows executable.' }
        $offset = [BitConverter]::ToUInt32($pe, 0x3c)
        if ($offset -gt $pe.Length - 6 -or [BitConverter]::ToUInt32($pe, $offset) -ne 0x4550 -or
            [BitConverter]::ToUInt16($pe, $offset + 4) -ne 0x8664) { throw 'Expected a Windows x64 executable.' }
        foreach ($licenseName in @('LICENSE-MIT', 'LICENSE-APACHE')) {
            $licenses = @($zip.Entries | Where-Object { $_.FullName -ceq $licenseName })
            if ($licenses.Count -ne 1 -or $licenses[0].Length -gt 1MB) { throw 'Missing or invalid release license.' }
        }
        foreach ($licenseName in @('LICENSE-MIT', 'LICENSE-APACHE')) {
            $licensePath = Join-Path $Directory $licenseName
            Assert-QqCliInstallPath $licensePath
            $reader = [IO.StreamReader]::new($zip.GetEntry($licenseName).Open())
            try { [IO.File]::WriteAllText($licensePath, $reader.ReadToEnd(), [Text.UTF8Encoding]::new($false)) }
            finally { $reader.Dispose() }
        }
        if (Test-Path -LiteralPath $target) {
            if ((Get-FileHash -LiteralPath $target).Hash -ne (Get-FileHash -LiteralPath $stage).Hash) {
                [IO.File]::Replace($stage, $target, [System.Management.Automation.Language.NullString]::Value)
            }
        } else { [IO.File]::Move($stage, $target) }
        return $target
    } finally {
        $zip.Dispose()
        if ($stage -and [IO.File]::Exists($stage)) { [IO.File]::Delete($stage) }
    }
}

function Install-QqCli {
    param([string] $Directory, [switch] $SkipPath)
    $ErrorActionPreference = 'Stop'
    if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT -or
        ($env:PROCESSOR_ARCHITECTURE -ne 'AMD64' -and $env:PROCESSOR_ARCHITEW6432 -ne 'AMD64')) {
        throw 'QQ CLI requires Windows x64.'
    }
    [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
    try { $release = Get-QqCliRelease } catch {
        throw "Cannot retrieve the latest QQ CLI release. Check https://github.com/noctisynth/qq-cli/releases and network/API limits. $($_.Exception.Message)"
    }
    $temp = Join-Path ([IO.Path]::GetTempPath()) ('qq-cli-install-' + [Guid]::NewGuid().ToString('N'))
    $null = [IO.Directory]::CreateDirectory($temp)
    $archive = Join-Path $temp $release.Archive.name
    $checksums = Join-Path $temp 'SHA256SUMS'
    try {
        Invoke-WebRequest -UseBasicParsing -Uri $release.Archive.browser_download_url -OutFile $archive -TimeoutSec 300 -ErrorAction Stop
        Invoke-WebRequest -UseBasicParsing -Uri $release.Checksums.browser_download_url -OutFile $checksums -TimeoutSec 60 -ErrorAction Stop
        $target = Install-QqCliArchive -Archive $archive -Checksums $checksums -Directory $Directory
        $installedDirectory = [IO.Path]::GetDirectoryName($target)
        if (-not $SkipPath) {
            $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
            $newPath = Merge-QqCliPath -Existing $userPath -Directory $installedDirectory
            if ($newPath -cne $userPath) { [Environment]::SetEnvironmentVariable('Path', $newPath, 'User') }
            $env:Path = Merge-QqCliPath -Existing $env:Path -Directory $installedDirectory
        }
        [pscustomobject]@{ executable = $target; asset = $release.Archive.name; pathUpdated = (-not $SkipPath) }
        Write-Host 'QQ CLI installed. Run qq-cli --help. Existing agent processes may need their PATH refreshed or restarted.'
    } finally {
        # Delete only files created by this invocation; never recursively remove a supplied directory.
        foreach ($file in @($archive, $checksums)) { if ([IO.File]::Exists($file)) { [IO.File]::Delete($file) } }
        [IO.Directory]::Delete($temp)
    }
}

if ($MyInvocation.InvocationName -ne '.') {
    Install-QqCli -Directory $InstallDirectory -SkipPath:$NoPath
}
