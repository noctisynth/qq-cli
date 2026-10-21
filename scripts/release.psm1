Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Assert-RegularSource {
    param([string] $Path)
    $item = Get-Item -LiteralPath $Path
    if ($item.PSIsContainer -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
        throw 'Release inputs must be regular files.'
    }
}

function New-ReleasePackage {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][ValidatePattern('^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$')][string] $Version,
        [Parameter(Mandatory)][string] $Executable,
        [Parameter(Mandatory)][string] $Readme,
        [Parameter(Mandatory)][string] $SkillRoot,
        [Parameter(Mandatory)][string] $OutputDirectory
    )
    Assert-RegularSource $Executable
    Assert-RegularSource $Readme
    $pe = [IO.File]::ReadAllBytes($Executable)
    if ($pe.Length -lt 64 -or $pe[0] -ne 0x4d -or $pe[1] -ne 0x5a) { throw 'Invalid Windows executable.' }
    $offset = [BitConverter]::ToUInt32($pe, 0x3c)
    if ($offset -gt $pe.Length - 6 -or [BitConverter]::ToUInt32($pe, $offset) -ne 0x4550 -or
        [BitConverter]::ToUInt16($pe, $offset + 4) -ne 0x8664) { throw 'Expected a Windows x64 PE executable.' }
    $skill = Get-Item -LiteralPath $SkillRoot
    if (-not $skill.PSIsContainer -or ($skill.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
        throw 'Skill root must be a regular directory.'
    }
    Assert-RegularSource (Join-Path $SkillRoot 'SKILL.md')
    $sources = [ordered]@{ 'qq-cli.exe' = $Executable; 'README.md' = $Readme }
    foreach ($entry in Get-ChildItem -LiteralPath $SkillRoot -Recurse | Sort-Object FullName) {
        if ($entry.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Skill links are not release inputs.' }
        if ($entry.PSIsContainer) { continue }
        Assert-RegularSource $entry.FullName
        $relative = [IO.Path]::GetRelativePath($skill.FullName, $entry.FullName).Replace('\', '/')
        $sources["skills/qq-cli/$relative"] = $entry.FullName
    }
    $root = [IO.Path]::GetFullPath($OutputDirectory)
    $null = [IO.Directory]::CreateDirectory($root)
    $archiveName = "qq-cli-v$Version-windows-x86_64.zip"
    $archivePath = Join-Path $root $archiveName
    $checksumPath = Join-Path $root 'SHA256SUMS'
    if ((Test-Path -LiteralPath $archivePath) -or (Test-Path -LiteralPath $checksumPath)) {
        throw 'Release output already exists; choose a new directory.'
    }
    $file = [IO.File]::Open($archivePath, [IO.FileMode]::CreateNew)
    try {
        $zip = [IO.Compression.ZipArchive]::new($file, [IO.Compression.ZipArchiveMode]::Create, $true)
        try {
            foreach ($name in $sources.Keys) {
                $entry = $zip.CreateEntry($name, [IO.Compression.CompressionLevel]::Optimal)
                $entry.LastWriteTime = [DateTimeOffset]::new(2000, 1, 1, 0, 0, 0, [TimeSpan]::Zero)
                $sourceStream = [IO.File]::OpenRead($sources[$name])
                try {
                    $destinationStream = $entry.Open()
                    try { $sourceStream.CopyTo($destinationStream) } finally { $destinationStream.Dispose() }
                } finally { $sourceStream.Dispose() }
            }
        } finally { $zip.Dispose() }
    } finally { $file.Dispose() }
    $hash = (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash.ToLowerInvariant()
    $checksums = [IO.File]::Open($checksumPath, [IO.FileMode]::CreateNew)
    try {
        $bytes = [Text.Encoding]::ASCII.GetBytes("$hash  $archiveName`n")
        $checksums.Write($bytes, 0, $bytes.Length)
    } finally { $checksums.Dispose() }
    Test-ReleasePackage -Archive $archivePath -Checksums $checksumPath -ExpectedEntries @($sources.Keys)
    [pscustomobject]@{ Archive = $archivePath; Checksums = $checksumPath; Version = $Version; Entries = $sources.Count }
}

function Test-ReleasePackage {
    [CmdletBinding()]
    param([string] $Archive, [string] $Checksums, [string[]] $ExpectedEntries)
    $hash = (Get-FileHash -LiteralPath $Archive -Algorithm SHA256).Hash.ToLowerInvariant()
    $expected = "$hash  $([IO.Path]::GetFileName($Archive))`n"
    if ([IO.File]::ReadAllText($Checksums) -cne $expected) { throw 'Release checksum does not match.' }
    $zip = [IO.Compression.ZipFile]::OpenRead($Archive)
    try {
        $names = @($zip.Entries | ForEach-Object FullName)
        if ($names.Count -ne $ExpectedEntries.Count -or
            @($names | Where-Object { $_ -cnotin $ExpectedEntries }).Count -ne 0 -or
            @($names | Sort-Object -Unique).Count -ne $names.Count) { throw 'Unexpected release archive entries.' }
        foreach ($name in $names) {
            if ($name -notin @('qq-cli.exe', 'README.md') -and -not $name.StartsWith('skills/qq-cli/', [StringComparison]::Ordinal)) {
                throw 'Only the original CLI, README and skill belong in the release archive.'
            }
            if ($name -match '(^|/)(\.\.?|\.agents|work|runtime|target)(/|$)' -or $name.Contains('\')) {
                throw 'Unsafe release archive path.'
            }
        }
    } finally { $zip.Dispose() }
}

Export-ModuleMember -Function New-ReleasePackage, Test-ReleasePackage
