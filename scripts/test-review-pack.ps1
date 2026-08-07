#Requires -Version 5.1
<#
.SYNOPSIS
    Self-test for create-review-pack.ps1.

.DESCRIPTION
    Creates a temporary git repo, commits, logs, a fake exe, and a test summary,
    then exercises create-review-pack.ps1 to verify every output and validation.
    This is a standalone regression test suite — it does NOT run against the
    production Review.zip.
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$PackScript = Join-Path $ScriptDir "create-review-pack.ps1"

$Global:Failures = New-Object System.Collections.Generic.List[string]
$Global:TestCount = 0
$Global:PassCount = 0

function Write-Info {
    param([string]$Message)
    Write-Host "[info] $Message"
}

function Assert-Equal {
    param($Expected, $Actual, [string]$Label)
    $Global:TestCount++
    if ($Expected -eq $Actual) {
        $Global:PassCount++
        Write-Host "[PASS] $Label"
    } else {
        $Global:Failures.Add("$Label : expected '$Expected', got '$Actual'")
        Write-Host "[FAIL] $Label : expected '$Expected', got '$Actual'"
    }
}

function Assert-True {
    param([bool]$Condition, [string]$Label)
    $Global:TestCount++
    if ($Condition) {
        $Global:PassCount++
        Write-Host "[PASS] $Label"
    } else {
        $Global:Failures.Add("$Label : expected true, got false")
        Write-Host "[FAIL] $Label"
    }
}

function Assert-FileExists {
    param([string]$Path, [string]$Label)
    $Global:TestCount++
    if (Test-Path -LiteralPath $Path -PathType Leaf) {
        $Global:PassCount++
        Write-Host "[PASS] $Label"
    } else {
        $Global:Failures.Add("$Label : file not found: $Path")
        Write-Host "[FAIL] $Label : file not found: $Path"
    }
}

function New-Utf8File {
    param([string]$Path, [string]$Content)
    $utf8 = [System.Text.UTF8Encoding]::new($false)
    [System.IO.File]::WriteAllText($Path, $Content, $utf8)
}

function Read-Utf8File {
    param([string]$Path)
    $utf8 = [System.Text.UTF8Encoding]::new($false)
    return [System.IO.File]::ReadAllText($Path, $utf8)
}

# ---- Setup: temporary environment ----

$RootTemp = Join-Path ([System.IO.Path]::GetTempPath()) "quackquota-review-test-$(Get-Date -Format 'yyyyMMddHHmmss')"
New-Item -ItemType Directory -Path $RootTemp -Force | Out-Null
Write-Info "Test root: $RootTemp"

$TestRepo = Join-Path $RootTemp "repo"
$TestLogs = Join-Path $RootTemp "logs"
$TestSummary = Join-Path $RootTemp "test-summary.md"
$TestExe = Join-Path $RootTemp "fake.exe"
$TestOutput = Join-Path $RootTemp "output"

New-Item -ItemType Directory -Path $TestLogs -Force | Out-Null
New-Item -ItemType Directory -Path $TestOutput -Force | Out-Null

Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem

# ---- Create minimal git repo ----

Write-Info "Creating test git repo: $TestRepo"
New-Item -ItemType Directory -Path $TestRepo -Force | Out-Null
git -C $TestRepo init 2>&1 | Out-Null
git -C $TestRepo config user.email "test@quackquota.local" 2>&1 | Out-Null
git -C $TestRepo config user.name "QuackQuota Test" 2>&1 | Out-Null

"base content" | Out-File -FilePath (Join-Path $TestRepo "file.txt") -Encoding utf8
git -C $TestRepo add file.txt 2>&1 | Out-Null
git -C $TestRepo commit -m "base commit" 2>&1 | Out-Null
$BaseHash = git -C $TestRepo rev-parse HEAD 2>&1
$BaseHash = $BaseHash.Trim()
Write-Info "Base: $BaseHash"

"previous content" | Out-File -FilePath (Join-Path $TestRepo "file.txt") -Encoding utf8
"extra file" | Out-File -FilePath (Join-Path $TestRepo "extra.txt") -Encoding utf8
git -C $TestRepo add file.txt extra.txt 2>&1 | Out-Null
git -C $TestRepo commit -m "previous head commit" 2>&1 | Out-Null
$PreviousHash = git -C $TestRepo rev-parse HEAD 2>&1
$PreviousHash = $PreviousHash.Trim()
Write-Info "PreviousHead: $PreviousHash"

"head content" | Out-File -FilePath (Join-Path $TestRepo "file.txt") -Encoding utf8
"another file" | Out-File -FilePath (Join-Path $TestRepo "another.txt") -Encoding utf8
git -C $TestRepo add file.txt another.txt 2>&1 | Out-Null
git -C $TestRepo commit -m "final head commit" 2>&1 | Out-Null
$HeadHash = git -C $TestRepo rev-parse HEAD 2>&1
$HeadHash = $HeadHash.Trim()
Write-Info "HEAD: $HeadHash"

"log line 1`r`nlog line 2`r`nlog line 3" | Out-File -FilePath (Join-Path $TestLogs "test-run-1.log") -Encoding utf8
"error: something failed`r`nwarning: check this" | Out-File -FilePath (Join-Path $TestLogs "test-run-2.log") -Encoding utf8
Write-Info "Created 2 test log files"

$summaryContent = @"
# Test Summary

## Results

- Syntax check: PASS
- Unit tests: PASS
- Integration: PASS

## Details

All tests passed successfully.
"@
New-Utf8File -Path $TestSummary -Content $summaryContent
Write-Info "Created test-summary.md"

$mz = [byte[]]@(0x4D, 0x5A, 0x00, 0x01, 0x02, 0x03)
[System.IO.File]::WriteAllBytes($TestExe, $mz)
Write-Info "Created fake.exe with MZ header"

# ============================================================================
# Test 1: Full pack (Push-Location TestRepo for BLOCK-1 success)
# ============================================================================

Write-Info "=== Test 1: Full pack ==="

$origDir = Get-Location
Push-Location -LiteralPath $TestRepo
try {
    $result1 = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $PackScript `
        -Repo $TestRepo `
        -Base $BaseHash `
        -TaskName "TestTask" `
        -LogsDirectory $TestLogs `
        -TestSummaryPath $TestSummary `
        -OutputRoot $TestOutput `
        -NotExecuted "integration-tests,visual-regression" `
        -Force 2>&1
    $exit1 = $LASTEXITCODE
} finally {
    Pop-Location
}
Assert-Equal -Expected 0 -Actual $exit1 -Label "Test 1 exit code"
$resultText1 = $result1 -join "`n"
Assert-True -Condition ($resultText1 -match "ReviewZip=") -Label "Test 1 ReviewZip output"
Assert-True -Condition ($resultText1 -match "ReviewZipSHA256=") -Label "Test 1 ReviewZipSHA256 output"
Assert-True -Condition ($resultText1 -match "ExitCode=0") -Label "Test 1 ExitCode=0"

$zipPath1 = Join-Path $TestOutput "QuackQuota-TestTask-Review.zip"
Assert-FileExists -Path $zipPath1 -Label "Test 1 ZIP exists"

# 1a: no backslashes
Write-Info "--- Test 1a: no backslash entries ---"
$za1 = $null; $zipFs1 = $null
try {
    $zipFs1 = [System.IO.File]::OpenRead($zipPath1)
    $za1 = New-Object System.IO.Compression.ZipArchive($zipFs1, [System.IO.Compression.ZipArchiveMode]::Read)
    $bc = 0
    foreach ($entry in $za1.Entries) { if ($entry.FullName -match '\\') { $bc++ } }
    Assert-Equal -Expected 0 -Actual $bc -Label "Test 1a no backslash entries"
} finally {
    if ($za1) { $za1.Dispose() }
    if ($zipFs1) { $zipFs1.Dispose() }
}

$extractDir1 = Join-Path $TestOutput "extract1"
if (Test-Path -LiteralPath $extractDir1) { Remove-Item -LiteralPath $extractDir1 -Recurse -Force }
[System.IO.Compression.ZipFile]::ExtractToDirectory($zipPath1, $extractDir1)

Assert-FileExists -Path (Join-Path $extractDir1 "manifest.md") -Label "Test 1 manifest.md"
Assert-FileExists -Path (Join-Path $extractDir1 "manifest.json") -Label "Test 1 manifest.json"
Assert-FileExists -Path (Join-Path $extractDir1 "git-status.txt") -Label "Test 1 git-status.txt"
Assert-FileExists -Path (Join-Path $extractDir1 "git-head.txt") -Label "Test 1 git-head.txt"
Assert-FileExists -Path (Join-Path $extractDir1 "git-log.txt") -Label "Test 1 git-log.txt"
Assert-FileExists -Path (Join-Path $extractDir1 "changed-files.txt") -Label "Test 1 changed-files.txt"
Assert-FileExists -Path (Join-Path $extractDir1 "full-stat.txt") -Label "Test 1 full-stat.txt"
Assert-FileExists -Path (Join-Path $extractDir1 "full.patch") -Label "Test 1 full.patch"
Assert-FileExists -Path (Join-Path $extractDir1 "head.zip") -Label "Test 1 head.zip"
Assert-FileExists -Path (Join-Path $extractDir1 "test-summary.md") -Label "Test 1 test-summary.md"
Assert-FileExists -Path (Join-Path $extractDir1 "logs\test-run-1.log") -Label "Test 1 logs/test-run-1.log"
Assert-FileExists -Path (Join-Path $extractDir1 "logs\test-run-2.log") -Label "Test 1 logs/test-run-2.log"

# multiline checks
$c1 = Read-Utf8File (Join-Path $extractDir1 "changed-files.txt")
Assert-True ( (@($c1 -split "`n" | Where-Object {$_})).Count -ge 2 ) "Test 1b changed-files.txt multiline"
$g1 = Read-Utf8File (Join-Path $extractDir1 "git-log.txt")
Assert-True ( (@($g1 -split "`n" | Where-Object {$_})).Count -ge 2 ) "Test 1c git-log.txt multiline"
$s1 = Read-Utf8File (Join-Path $extractDir1 "full-stat.txt")
Assert-True ( (@($s1 -split "`n" | Where-Object {$_})).Count -ge 2 ) "Test 1d full-stat.txt multiline"

# NotExecuted
$md1 = Read-Utf8File (Join-Path $extractDir1 "manifest.md")
Assert-True ($md1 -match "integration-tests") "Test 1e manifest.md NotExecuted integration-tests"
Assert-True ($md1 -match "visual-regression") "Test 1e manifest.md NotExecuted visual-regression"

$mj1 = Read-Utf8File (Join-Path $extractDir1 "manifest.json") | ConvertFrom-Json
$ne1 = @($mj1.not_executed)
Assert-True ($ne1 -contains "integration-tests") "Test 1e manifest.json not_executed integration-tests"
Assert-True ($ne1 -contains "visual-regression") "Test 1e manifest.json not_executed visual-regression"
Assert-Equal 2 $ne1.Count "Test 1e not_executed count = 2"

Assert-Equal $TestRepo $mj1.repo "Test 1f manifest.repo matches"

# head.zip
$hfs = [System.IO.File]::OpenRead((Join-Path $extractDir1 "head.zip"))
$hza = New-Object System.IO.Compression.ZipArchive($hfs, [System.IO.Compression.ZipArchiveMode]::Read)
Assert-True ($hza.Entries.Count -gt 0) "Test 1g head.zip openable ($($hza.Entries.Count) entries)"
$hza.Dispose(); $hfs.Dispose()

Assert-True (-not (Test-Path (Join-Path $extractDir1 "incremental.patch"))) "Test 1 no incremental.patch"
Assert-True (-not (Test-Path (Join-Path $extractDir1 "incremental-stat.txt"))) "Test 1 no incremental-stat.txt"

Assert-True (-not (Join-Path $TestOutput "x").StartsWith($TestRepo + '\')) "Test 1 ZIP outside repo"

Remove-Item -LiteralPath $extractDir1 -Recurse -Force -ErrorAction SilentlyContinue

# ============================================================================
# Test 1h: BLOCK-1 PWD != Repo must fail
# ============================================================================

Write-Info "=== Test 1h: BLOCK-1 PWD != Repo ==="
$notRepoDir = Join-Path $RootTemp "not-repo"
New-Item -ItemType Directory -Path $notRepoDir -Force | Out-Null
Push-Location -LiteralPath $notRepoDir
try {
    $wrongZip = Join-Path $TestOutput "QuackQuota-WrongDir-Review.zip"
    if (Test-Path $wrongZip) { Remove-Item $wrongZip -Force }
    $null = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $PackScript `
        -Repo $TestRepo -Base $BaseHash -TaskName "WrongDir" `
        -LogsDirectory $TestLogs -TestSummaryPath $TestSummary `
        -OutputRoot $TestOutput -Force *>&1
    $exitH = $LASTEXITCODE
} finally {
    Pop-Location
}
Assert-True ($exitH -ne 0) "Test 1h PWD != Repo fails (exit != 0)"
Assert-True (-not (Test-Path $wrongZip)) "Test 1h no ZIP when PWD mismatch"
$sd = @(Get-ChildItem $TestOutput -Dir -Filter ".quackquota-staging-*" -EA SilentlyContinue)
Assert-True ($sd.Count -eq 0) "Test 1h no staging residue"

# ============================================================================
# Test 2: Full pack with PreviousHead and Artifact
# ============================================================================

Write-Info "=== Test 2: Full pack with PreviousHead and Artifact ==="
Push-Location -LiteralPath $TestRepo
try {
    $result2 = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $PackScript `
        -Repo $TestRepo -Base $BaseHash -PreviousHead $PreviousHash `
        -TaskName "TestWithArtifact" -LogsDirectory $TestLogs `
        -TestSummaryPath $TestSummary -OutputRoot $TestOutput `
        -ArtifactPath $TestExe -Force 2>&1
    $exit2 = $LASTEXITCODE
} finally {
    Pop-Location
}
Assert-Equal 0 $exit2 "Test 2 exit code"

$zipPath2 = Join-Path $TestOutput "QuackQuota-TestWithArtifact-Review.zip"
Assert-FileExists $zipPath2 "Test 2 ZIP exists"

$extractDir2 = Join-Path $TestOutput "extract2"
[System.IO.Compression.ZipFile]::ExtractToDirectory($zipPath2, $extractDir2)
Assert-FileExists (Join-Path $extractDir2 "incremental.patch") "Test 2 incremental.patch"
Assert-FileExists (Join-Path $extractDir2 "incremental-stat.txt") "Test 2 incremental-stat.txt"
Assert-FileExists (Join-Path $extractDir2 "artifact-info.txt") "Test 2 artifact-info.txt"
Assert-True ((Get-Item (Join-Path $extractDir2 "incremental.patch")).Length -gt 0) "Test 2 incremental.patch has content"

# artifact-info.txt
$ai2 = Read-Utf8File (Join-Path $extractDir2 "artifact-info.txt")
Assert-True ($ai2 -match "SHA256=") "Test 2a artifact-info has SHA256"
Assert-True ($ai2 -match "4D 5A \(MZ\)") "Test 2a artifact-info has MZ header"

# manifest.json artifact
$mj2 = (Read-Utf8File (Join-Path $extractDir2 "manifest.json")) | ConvertFrom-Json
Assert-True ($mj2.artifact -ne $null) "Test 2b manifest.json has artifact section"
Assert-True ($mj2.artifact.sha256 -and $mj2.artifact.sha256.Length -gt 0) "Test 2b manifest.json artifact SHA-256"
Assert-Equal "fake.exe" $mj2.artifact.file_name "Test 2b manifest.json artifact file_name"
Assert-True ($mj2.artifact.size -gt 0) "Test 2b manifest.json artifact size > 0"
Assert-True ($mj2.artifact.file_header -match "MZ") "Test 2b manifest.json artifact file_header"

# FIX-3C: manifest.md Artifact section — explicit field verification
$md2 = Read-Utf8File (Join-Path $extractDir2 "manifest.md")
Assert-True ($md2 -match "## Artifact")                "Test 2c-0 Artifact section header"
Assert-True ($md2 -match '\*\*Path\*\*')                "Test 2c-1 Artifact: Path"
Assert-True ($md2 -match '\*\*File Name\*\*')           "Test 2c-2 Artifact: File Name"
Assert-True ($md2 -match '\*\*Size\*\*')                "Test 2c-3 Artifact: Size"
Assert-True ($md2 -match '\*\*Creation Time\*\*')       "Test 2c-4 Artifact: Creation Time"
Assert-True ($md2 -match '\*\*Last Write Time\*\*')     "Test 2c-5 Artifact: Last Write Time"
Assert-True ($md2 -match '\*\*SHA-256\*\*')             "Test 2c-6 Artifact: SHA-256"
Assert-True ($md2 -match '\*\*File Header\*\*')         "Test 2c-7 Artifact: File Header"
# Verify actual values present (not just field labels)
Assert-True ($md2 -match "fake\.exe")                   "Test 2c-8 Artifact: fake.exe present"
Assert-True ($md2 -match "MZ")                          "Test 2c-9 Artifact: MZ header text present"
Assert-True ($md2 -match "\d+ bytes")                   "Test 2c-10 Artifact: size with bytes unit"

Assert-True ($md2 -match [regex]::Escape($PreviousHash)) "Test 2 manifest has PreviousHead"
Assert-True ($md2 -match "Yes") "Test 2 manifest Has Incremental = Yes"

Remove-Item -LiteralPath $extractDir2 -Recurse -Force -ErrorAction SilentlyContinue

# ============================================================================
# Test 3: Dirty workspace fails
# ============================================================================

Write-Info "=== Test 3: Dirty workspace ==="
$dirtyRepo = Join-Path $RootTemp "dirty-repo"
New-Item -ItemType Directory -Path $dirtyRepo -Force | Out-Null
git -C $dirtyRepo init 2>&1 | Out-Null
git -C $dirtyRepo config user.email "t@t.l"; git -C $dirtyRepo config user.name "T"
"clean" | Out-File (Join-Path $dirtyRepo "f.txt") -Encoding utf8
git -C $dirtyRepo add f.txt; git -C $dirtyRepo commit -m "init"
$dirtyBase = (git -C $dirtyRepo rev-parse HEAD).Trim()
"dirty" | Out-File (Join-Path $dirtyRepo "f.txt") -Encoding utf8
$dirtyZip = Join-Path $TestOutput "QuackQuota-DirtyTask-Review.zip"
if (Test-Path $dirtyZip) { Remove-Item $dirtyZip -Force }
Push-Location $dirtyRepo
try {
    $null = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $PackScript `
        -Repo $dirtyRepo -Base $dirtyBase -TaskName "DirtyTask" `
        -LogsDirectory $TestLogs -TestSummaryPath $TestSummary `
        -OutputRoot $TestOutput -Force *>&1
    $exit3 = $LASTEXITCODE
} finally { Pop-Location }
Assert-True ($exit3 -ne 0) "Test 3 dirty workspace fails"
Assert-True (-not (Test-Path $dirtyZip)) "Test 3a no leftover ZIP"

# ============================================================================
# Test 4: Missing logs dir
# ============================================================================
Write-Info "=== Test 4: Missing logs ==="
Push-Location $TestRepo
try {
    $null = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $PackScript `
        -Repo $TestRepo -Base $BaseHash -TaskName "MissingLogs" `
        -LogsDirectory (Join-Path $RootTemp "nologs") -TestSummaryPath $TestSummary `
        -OutputRoot $TestOutput -Force *>&1
    $exit4 = $LASTEXITCODE
} finally { Pop-Location }
Assert-True ($exit4 -ne 0) "Test 4 missing logs fails"

# ============================================================================
# Test 5: Missing test summary
# ============================================================================
Write-Info "=== Test 5: Missing summary ==="
Push-Location $TestRepo
try {
    $null = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $PackScript `
        -Repo $TestRepo -Base $BaseHash -TaskName "MissingSummary" `
        -LogsDirectory $TestLogs -TestSummaryPath (Join-Path $RootTemp "nosummary.md") `
        -OutputRoot $TestOutput -Force *>&1
    $exit5 = $LASTEXITCODE
} finally { Pop-Location }
Assert-True ($exit5 -ne 0) "Test 5 missing summary fails"

# ============================================================================
# Test 6: Manifest JSON
# ============================================================================
Write-Info "=== Test 6: Manifest JSON ==="
$zfs6 = [System.IO.File]::OpenRead($zipPath1)
$za6 = New-Object System.IO.Compression.ZipArchive($zfs6, [System.IO.Compression.ZipArchiveMode]::Read)
$rd6 = New-Object System.IO.StreamReader($za6.GetEntry("manifest.json").Open(), [System.Text.Encoding]::UTF8)
try { $p6 = ($rd6.ReadToEnd() | ConvertFrom-Json) } catch { $p6 = $null }
$rd6.Dispose(); $za6.Dispose(); $zfs6.Dispose()
Assert-Equal "TestTask" $p6.task_name "Test 6 task_name"
Assert-Equal $BaseHash $p6.base "Test 6 base"
Assert-Equal $HeadHash $p6.head "Test 6 head"
Assert-True ($p6.has_incremental -eq $false) "Test 6 has_incremental = false"

# ============================================================================
# Test 7: Log content preserved
# ============================================================================
Write-Info "=== Test 7: Log content ==="
$zfs7 = [System.IO.File]::OpenRead($zipPath1)
$za7 = New-Object System.IO.Compression.ZipArchive($zfs7, [System.IO.Compression.ZipArchiveMode]::Read)
$r7 = New-Object System.IO.StreamReader($za7.GetEntry("logs/test-run-1.log").Open(), [System.Text.Encoding]::UTF8)
$lc7 = $r7.ReadToEnd(); $r7.Dispose()
Assert-True ($lc7 -match "log line 1") "Test 7 log line 1"
Assert-True ($lc7 -match "log line 2") "Test 7 log line 2"
Assert-True ($lc7 -match "log line 3") "Test 7 log line 3"
$rts = New-Object System.IO.StreamReader($za7.GetEntry("test-summary.md").Open(), [System.Text.Encoding]::UTF8)
$tsc = $rts.ReadToEnd(); $rts.Dispose()
Assert-True ($tsc -match "All tests passed") "Test 7 test-summary content"
$za7.Dispose(); $zfs7.Dispose()

# ============================================================================
# Test 8: OutputRoot inside repo
# ============================================================================
Write-Info "=== Test 8: OutputRoot inside repo ==="
Push-Location $TestRepo
try {
    $null = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $PackScript `
        -Repo $TestRepo -Base $BaseHash -TaskName "InsideRepo" `
        -LogsDirectory $TestLogs -TestSummaryPath $TestSummary `
        -OutputRoot (Join-Path $TestRepo "inside") -Force *>&1
    $exit8 = $LASTEXITCODE
} finally { Pop-Location }
Assert-True ($exit8 -ne 0) "Test 8 OutputRoot inside repo fails"

# ============================================================================
# Test 9: Adjacent dir succeeds
# ============================================================================
Write-Info "=== Test 9: Adjacent dir ==="
$adj = "$TestRepo-adjacent"; New-Item -ItemType Directory -Path $adj -Force | Out-Null
Push-Location $TestRepo
try {
    $null = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $PackScript `
        -Repo $TestRepo -Base $BaseHash -TaskName "AdjacentDir" `
        -LogsDirectory $TestLogs -TestSummaryPath $TestSummary `
        -OutputRoot $adj -Force *>&1
    $exit9 = $LASTEXITCODE
} finally { Pop-Location }
Assert-Equal 0 $exit9 "Test 9 adjacent dir succeeds"
Assert-FileExists (Join-Path $adj "QuackQuota-AdjacentDir-Review.zip") "Test 9 ZIP created"
Remove-Item -LiteralPath $adj -Recurse -Force -ErrorAction SilentlyContinue

# ============================================================================
# Test 12: Empty NotExecuted
# ============================================================================
Write-Info "=== Test 12: Empty NotExecuted ==="
Push-Location $TestRepo
try {
    $null = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $PackScript `
        -Repo $TestRepo -Base $BaseHash -TaskName "EmptyNotExecuted" `
        -LogsDirectory $TestLogs -TestSummaryPath $TestSummary `
        -OutputRoot $TestOutput -Force *>&1
    $exit12 = $LASTEXITCODE
} finally { Pop-Location }
Assert-Equal 0 $exit12 "Test 12 empty NotExecuted succeeds"
$z12 = Join-Path $TestOutput "QuackQuota-EmptyNotExecuted-Review.zip"
$zfs12 = [System.IO.File]::OpenRead($z12)
$za12 = New-Object System.IO.Compression.ZipArchive($zfs12, [System.IO.Compression.ZipArchiveMode]::Read)
$r12 = New-Object System.IO.StreamReader($za12.GetEntry("manifest.json").Open(), [System.Text.Encoding]::UTF8)
$p12 = $r12.ReadToEnd() | ConvertFrom-Json; $r12.Dispose()
Assert-Equal 0 @($p12.not_executed).Count "Test 12 not_executed count = 0"
$rm12 = New-Object System.IO.StreamReader($za12.GetEntry("manifest.md").Open(), [System.Text.Encoding]::UTF8)
Assert-True (($rm12.ReadToEnd()) -match "\(none\)") "Test 12 manifest.md (none)"
$rm12.Dispose(); $za12.Dispose(); $zfs12.Dispose()

# ============================================================================
# Test 13: git-status.txt multiline
# ============================================================================
Write-Info "=== Test 13: git-status.txt multiline ==="
$zfs13 = [System.IO.File]::OpenRead($zipPath1)
$za13 = New-Object System.IO.Compression.ZipArchive($zfs13, [System.IO.Compression.ZipArchiveMode]::Read)
$r13 = New-Object System.IO.StreamReader($za13.GetEntry("git-status.txt").Open(), [System.Text.Encoding]::UTF8)
$gs13 = $r13.ReadToEnd(); $r13.Dispose()
Assert-True ((@($gs13 -split "`n" | Where-Object {$_})).Count -gt 0) "Test 13 git-status.txt has content"
Assert-True ($gs13 -match "##") "Test 13 git-status.txt branch header"
$za13.Dispose(); $zfs13.Dispose()

# ============================================================================
# Test 16: SelfTestFault "omit-required-entry" — real post-create validation failure (FIX-3B)
# ============================================================================
Write-Info "=== Test 16: SelfTestFault omit-required-entry ==="
$sfZip = Join-Path $TestOutput "QuackQuota-SelfTestFault-Review.zip"
if (Test-Path $sfZip) { Remove-Item $sfZip -Force }
Push-Location $TestRepo
try {
    $result16 = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $PackScript `
        -Repo $TestRepo -Base $BaseHash -TaskName "SelfTestFault" `
        -LogsDirectory $TestLogs -TestSummaryPath $TestSummary `
        -OutputRoot $TestOutput -SelfTestFault "omit-required-entry" -Force 2>&1
    $exit16 = $LASTEXITCODE
} finally { Pop-Location }
$text16 = $result16 -join "`n"

Assert-True ($exit16 -ne 0) "Test 16a SelfTestFault fails (exit != 0)"
Assert-True ($text16 -notmatch "ExitCode=0") "Test 16b no ExitCode=0"

# FIX-3B: verify post-create validation path was exercised
Assert-True ($text16 -match "ZIP created")                "Test 16c ZIP was created"
Assert-True ($text16 -match "omitting entry:")            "Test 16d fault injection logged"
Assert-True ($text16 -match "Missing required entry:")    "Test 16e validation reported missing entry"
Assert-True ($text16 -match "Invalid ZIP deleted")        "Test 16f invalid ZIP deleted message"

Assert-True (-not (Test-Path $sfZip)) "Test 16g no ZIP left"
$sd16 = @(Get-ChildItem $TestOutput -Dir -Filter ".quackquota-staging-*" -EA SilentlyContinue)
Assert-True ($sd16.Count -eq 0) "Test 16h no staging residue"

# ============================================================================
# Test 17: SelfTestFault "staging-create-fault" (FIX-3A, strengthened FIX-4A)
# ============================================================================
Write-Info "=== Test 17: SelfTestFault staging-create-fault ==="
$scfZip = Join-Path $TestOutput "QuackQuota-StagingCreateFault-Review.zip"
if (Test-Path $scfZip) { Remove-Item $scfZip -Force }
Push-Location $TestRepo
$exit17 = 0; $result17 = @()
try {
    $prevEAP = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        $result17 = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $PackScript `
            -Repo $TestRepo -Base $BaseHash -TaskName "StagingCreateFault" `
            -LogsDirectory $TestLogs -TestSummaryPath $TestSummary `
            -OutputRoot $TestOutput -SelfTestFault "staging-create-fault" -Force *>&1
        $exit17 = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $prevEAP
    }
} finally { Pop-Location }
$text17 = $result17 -join "`n"

# FIX-4A: prove the fault was actually triggered, not just ExitCode != 0
# Note: Write-Host goes to host console, not captured by *>&1 on PS 5.1.
# The throw error text IS captured — verify that.
Assert-True ($exit17 -ne 0)                                "Test 17a staging-create-fault fails (exit != 0)"
Assert-True ($text17 -match 'SelfTestFault.*staging')      "Test 17b fault injection text captured in output"
Assert-True ($text17 -match 'staging-create-fault')         "Test 17c throw error message captured"
Assert-True ($text17 -notmatch 'ExitCode=0')                "Test 17d no ExitCode=0"
Assert-True (-not (Test-Path $scfZip))                       "Test 17e no success ZIP after staging fault"
$sd17 = @(Get-ChildItem $TestOutput -Dir -Filter ".quackquota-staging-*" -EA SilentlyContinue)
Assert-True ($sd17.Count -eq 0)                             "Test 17f no staging residue after staging fault"

# ============================================================================
# Test 15: Non-git-root Repo path fails
# ============================================================================
Write-Info "=== Test 15: Non-git-root Repo ==="
$notARepo = Join-Path $RootTemp "not-a-repo"
New-Item -ItemType Directory -Path $notARepo -Force | Out-Null
$exit15 = 0
Push-Location $notARepo
try {
    try { $null = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $PackScript -Repo $notARepo -Base $BaseHash -TaskName "NotARepo" -LogsDirectory $TestLogs -TestSummaryPath $TestSummary -OutputRoot $TestOutput -Force *>&1; $exit15 = $LASTEXITCODE } catch { $exit15 = 1 }
} finally { Pop-Location }
Assert-True ($exit15 -ne 0) "Test 15 non-git-root Repo fails"

# ============================================================================
# Final cleanup
# ============================================================================
Assert-True (Test-Path $RootTemp) "Test 14 temp dir exists before cleanup"
Remove-Item -LiteralPath $RootTemp -Recurse -Force -ErrorAction SilentlyContinue
Write-Info "Test environment cleaned"
Assert-True (-not (Test-Path $RootTemp)) "Test 14 final temp dir cleaned"

Write-Host ""
Write-Host "========================================"
Write-Host "  Review Pack Self-Test Results"
Write-Host "========================================"
Write-Host "Tests Run   : $TestCount"
Write-Host "Passed      : $PassCount"
Write-Host "Failed      : $($Global:Failures.Count)"
Write-Host ""

if ($Global:Failures.Count -gt 0) {
    Write-Host "FAILURES:"
    foreach ($f in $Global:Failures) { Write-Host "  - $f" }
    Write-Host ""
    exit 1
}

Write-Host "All tests passed."
exit 0
