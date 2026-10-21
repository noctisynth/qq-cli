$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Import-Module (Join-Path $PSScriptRoot 'release.psm1') -Force
$verificationRoot = [IO.Path]::GetFullPath((Join-Path $projectRoot '.verification'))
$testRoot = Join-Path $verificationRoot ("release-tests-" + [Guid]::NewGuid().ToString('N'))
$null = [IO.Directory]::CreateDirectory($testRoot)

function Assert-Rejected {
    param([scriptblock] $Operation)
    $rejected = $false
    try { & $Operation | Out-Null } catch { $rejected = $true }
    if (-not $rejected) { throw 'Expected invalid release input to be rejected.' }
}

Push-Location -LiteralPath $projectRoot
try {
    $executable = Join-Path $testRoot 'synthetic.exe'
    $pe = [byte[]]::new(256)
    $pe[0] = 0x4d; $pe[1] = 0x5a; $pe[0x3c] = 128
    $pe[128] = 0x50; $pe[129] = 0x45; $pe[132] = 0x64; $pe[133] = 0x86
    [IO.File]::WriteAllBytes($executable, $pe)
    $readme = Join-Path $testRoot 'README.md'
    [IO.File]::WriteAllText($readme, 'Synthetic distribution documentation')
    $skill = Join-Path $testRoot 'skill'
    $null = [IO.Directory]::CreateDirectory((Join-Path $skill 'references'))
    [IO.File]::WriteAllText((Join-Path $skill 'SKILL.md'), 'Synthetic skill')
    [IO.File]::WriteAllText((Join-Path $skill 'references/read.md'), 'Synthetic reference')
    foreach ($license in @('LICENSE-MIT', 'LICENSE-APACHE')) { [IO.File]::WriteAllText((Join-Path $testRoot $license), 'Synthetic license') }
    $parameters = @{ LicenseRoot = $testRoot; Version = '1.2.3'; Executable = $executable; Readme = $readme; SkillRoot = $skill; OutputDirectory = (Join-Path $testRoot 'output') }
    $result = New-ReleasePackage @parameters
    $entries = @('qq-cli.exe', 'README.md', 'LICENSE-MIT', 'LICENSE-APACHE', 'skills/qq-cli/SKILL.md', 'skills/qq-cli/references/read.md')
    Test-ReleasePackage -Archive $result.Archive -Checksums $result.Checksums -ExpectedEntries $entries
    $before = (Get-FileHash -LiteralPath $result.Archive).Hash
    Assert-Rejected { New-ReleasePackage @parameters }
    if ((Get-FileHash -LiteralPath $result.Archive).Hash -cne $before) { throw 'Existing output was overwritten.' }
    $checksums = [IO.File]::ReadAllText($result.Checksums)
    [IO.File]::WriteAllText($result.Checksums, 'tampered')
    Assert-Rejected { Test-ReleasePackage -Archive $result.Archive -Checksums $result.Checksums -ExpectedEntries $entries }
    [IO.File]::WriteAllText($result.Checksums, $checksums)
    $pe[132] = 0x4c; $pe[133] = 1
    [IO.File]::WriteAllBytes($executable, $pe)
    $parameters.OutputDirectory = Join-Path $testRoot 'wrong-machine'
    Assert-Rejected { New-ReleasePackage @parameters }
    $archive = [IO.Compression.ZipFile]::Open($result.Archive, [IO.Compression.ZipArchiveMode]::Update)
    try { $null = $archive.CreateEntry('skills/qq-cli/../private.json') } finally { $archive.Dispose() }
    $hash = (Get-FileHash -LiteralPath $result.Archive).Hash.ToLowerInvariant()
    [IO.File]::WriteAllText($result.Checksums, "$hash  $([IO.Path]::GetFileName($result.Archive))`n")
    Assert-Rejected { Test-ReleasePackage -Archive $result.Archive -Checksums $result.Checksums -ExpectedEntries ($entries + 'skills/qq-cli/../private.json') }

    # Exercise the real post-version command in an isolated copy; never version this repository.
    $fixture = Join-Path $testRoot 'version-fixture'
    $null = [IO.Directory]::CreateDirectory((Join-Path $fixture 'src'))
    [IO.File]::WriteAllText((Join-Path $fixture 'src/lib.rs'), '')
    $manifest = [IO.File]::ReadAllText((Join-Path $projectRoot 'Cargo.toml'))
    $manifest = ($manifest -replace '(?m)^version = "[^"]+"', 'version = "99.0.0"') + "`n[workspace]`n"
    [IO.File]::WriteAllText((Join-Path $fixture 'Cargo.toml'), $manifest)
    $lock = [IO.File]::ReadAllText((Join-Path $projectRoot 'Cargo.lock'))
    [IO.File]::WriteAllText((Join-Path $fixture 'Cargo.lock'), $lock)
    & cargo update --workspace --offline --manifest-path (Join-Path $fixture 'Cargo.toml')
    if ($LASTEXITCODE -ne 0) { throw 'Isolated workspace version synchronization failed.' }
    $expectedLock = $lock -replace '(?m)(name = "qq-cli"\r?\nversion = ")[^"]+("$)', '${1}99.0.0${2}'
    if ($expectedLock -ceq $lock -or [IO.File]::ReadAllText((Join-Path $fixture 'Cargo.lock')) -cne $expectedLock) {
        throw 'Post-version synchronization changed more than the workspace version.'
    }
    Write-Output 'Release archive, overwrite protection, checksum, PE target and version-lock checks: OK'
} finally {
    Pop-Location
    $resolvedTestRoot = [IO.Path]::GetFullPath($testRoot)
    if (-not $resolvedTestRoot.StartsWith($verificationRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Refusing to clean a fixture outside the verification directory.'
    }
    Remove-Item -LiteralPath $resolvedTestRoot -Recurse -Force
}
