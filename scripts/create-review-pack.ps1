#Requires -Version 5.1
<#
.SYNOPSIS
    Create a standard QuackQuota review pack ZIP for GPT-5.6 code review.

.DESCRIPTION
    Generates a single ZIP file containing all evidence needed for an
    automated code review: full/incremental patches, git archive, logs,
    test summary, and a manifest.

    Output file: QuackQuota-<TaskName>-Review.zip

.PARAMETER Repo
    Path to the Git repository. The script verifies that the current
    working directory equals this path.

.PARAMETER Base
    Base commit hash. Must be an ancestor of HEAD.

.PARAMETER TaskName
    Human-readable task name. Sanitised for the ZIP filename.

.PARAMETER LogsDirectory
    Directory containing raw test/validation log files.

.PARAMETER TestSummaryPath
    Path to the test-summary.md file.

.PARAMETER OutputRoot
    Directory where the ZIP will be written. Must be outside the repo.

.PARAMETER PreviousHead
    Optional previous HEAD for an incremental patch.

.PARAMETER ArtifactPath
    Optional path to a built .exe for artifact-info.txt.

.PARAMETER NotExecuted
    Optional list of test/validation items that were not executed.
    When calling via powershell.exe -File, use comma-separated values:
      -NotExecuted "item1,item2"

.PARAMETER SelfTestFault
    INTERNAL USE ONLY. Injects a controlled fault for testing.
    Values: "", "omit-required-entry", "staging-create-fault".
    Must not be used in production.

.PARAMETER Force
    Overwrite an existing ZIP with the same name.
#>

param(
    [Parameter(Mandatory = $true)][string]$Repo,
    [Parameter(Mandatory = $true)][string]$Base,
    [Parameter(Mandatory = $true)][string]$TaskName,
    [Parameter(Mandatory = $true)][string]$LogsDirectory,
    [Parameter(Mandatory = $true)][string]$TestSummaryPath,
    [Parameter(Mandatory = $true)][string]$OutputRoot,
    [string]$PreviousHead = "",
    [string]$ArtifactPath = "",
    [string[]]$NotExecuted = @(),
    [string]$SelfTestFault = "",
    [switch]$Force
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ExitCode = 1

$script:HadError = $false
$script:StagingCleanupFailed = $false

# ---- lifecycle variables (initialized to $null for safe finally blocks) ----
$Staging = $null
$zipFs = $null
$zipArchive = $null
$verifyFs = $null
$verifyArchive = $null

function Write-Info {
    param([string]$Message)
    Write-Host "[info] $Message"
}

function Write-Fail {
    param([string]$Message)
    Write-Host "[FAIL] $Message"
    if ($script:HadError) { return }
    $script:HadError = $true
}

function Write-Ok {
    param([string]$Message)
    Write-Host "[ok]   $Message"
}

function New-Utf8File {
    param([string]$Path, [string]$Content)
    $utf8 = [System.Text.UTF8Encoding]::new($false)
    [System.IO.File]::WriteAllText($Path, $Content, $utf8)
}

function Test-IsAncestor {
    param([string]$Ancestor, [string]$Descendant, [string]$RepoParam)
    $null = git -C $RepoParam merge-base --is-ancestor $Ancestor $Descendant 2>&1
    return ($LASTEXITCODE -eq 0)
}

function Get-SHA256 {
    param([string]$Path)
    $sha = [System.Security.Cryptography.SHA256]::Create()
    $stream = [System.IO.File]::OpenRead($Path)
    try {
        $hash = [System.BitConverter]::ToString($sha.ComputeHash($stream)).Replace("-", "").ToLowerInvariant()
        return $hash
    } finally {
        $stream.Dispose()
        $sha.Dispose()
    }
}

function Sanitise-TaskName {
    param([string]$Name)
    $sanitised = $Name -replace '[^a-zA-Z0-9._-]', '-'
    $sanitised = $sanitised -replace '-{2,}', '-'
    $sanitised = $sanitised.Trim('-')
    if ($sanitised.Length -eq 0) { $sanitised = "task" }
    return $sanitised
}

function Invoke-GitLines {
    param([string]$RepoPath, [string[]]$GitArgs)
    $lines = & git -C $RepoPath @GitArgs 2>&1
    if ($lines -is [array]) {
        return ($lines -join [System.Environment]::NewLine)
    }
    return [string]$lines
}

function Test-OutsideRepo {
    param([string]$OutputDir, [string]$RepoDirParam)
    $out = [System.IO.Path]::GetFullPath($OutputDir).TrimEnd('\', '/')
    $rp  = [System.IO.Path]::GetFullPath($RepoDirParam).TrimEnd('\', '/')
    if ($out -eq $rp) { return $false }
    if ($out.StartsWith($rp + '\', [System.StringComparison]::OrdinalIgnoreCase)) { return $false }
    if ($out.StartsWith($rp + '/', [System.StringComparison]::OrdinalIgnoreCase)) { return $false }
    return $true
}

# ---- Step 0: resolve paths ----

$Repo = [System.IO.Path]::GetFullPath($Repo)
$LogsDirectory = [System.IO.Path]::GetFullPath($LogsDirectory)
$TestSummaryPath = [System.IO.Path]::GetFullPath($TestSummaryPath)
$OutputRoot = [System.IO.Path]::GetFullPath($OutputRoot)

if ($PreviousHead) {
    $PreviousHead = $PreviousHead.Trim()
}
if ($ArtifactPath) {
    $ArtifactPath = [System.IO.Path]::GetFullPath($ArtifactPath)
}

if ($NotExecuted.Count -eq 1 -and $NotExecuted[0] -match ',') {
    $NotExecuted = @($NotExecuted[0] -split ',' | ForEach-Object { $_.Trim() } | Where-Object { $_ -ne '' })
}

if ($PreviousHead -and $PreviousHead -notmatch '^[0-9a-fA-F]{7,40}$') {
    Write-Fail "PreviousHead does not look like a commit SHA: '$PreviousHead'."
    exit $ExitCode
}

$validFaults = @("", "omit-required-entry", "staging-create-fault")
if ($SelfTestFault -notin $validFaults) {
    Write-Fail "Invalid SelfTestFault value: '$SelfTestFault'. Allowed: $($validFaults -join ', ')"
    exit $ExitCode
}

$SafeName = Sanitise-TaskName -Name $TaskName
$ZipName = "QuackQuota-${SafeName}-Review.zip"
$ZipPath = Join-Path $OutputRoot $ZipName

# ---- Step 1: validate ----

Write-Info "Validating inputs for review pack: $SafeName"

# BLOCK-1: PWD == Repo
$currentPwd = [System.IO.Path]::GetFullPath((Get-Location).Path)
if (-not $currentPwd.Equals($Repo, [System.StringComparison]::OrdinalIgnoreCase)) {
    Write-Fail "Working directory mismatch. Current: '$currentPwd', Expected (Repo): '$Repo'."
    exit $ExitCode
}
Write-Ok "Current directory matches Repo"

# BLOCK-1: Repo is consistent git root
$repoGitRoot = git -C $Repo rev-parse --show-toplevel 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Fail "Cannot determine git root for: $Repo"
    exit $ExitCode
}
$repoGitRoot = [System.IO.Path]::GetFullPath($repoGitRoot)
if (-not $repoGitRoot.Equals($Repo, [System.StringComparison]::OrdinalIgnoreCase)) {
    Write-Fail "Repo path mismatch: '$Repo' resolves to '$repoGitRoot'."
    exit $ExitCode
}
Write-Ok "Repo verified as valid git root"

if (-not (Test-Path -LiteralPath $Repo -PathType Container)) {
    Write-Fail "Repo path does not exist: $Repo"
    exit $ExitCode
}

$null = git -C $Repo rev-parse --git-dir 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Fail "Repo is not a Git workspace: $Repo"
    exit $ExitCode
}

$statusOutput = git -C $Repo status --porcelain 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Fail "Cannot read Git status"
    exit $ExitCode
}
if ($statusOutput) {
    Write-Fail "Working tree or index is not clean."
    Write-Host $statusOutput
    exit $ExitCode
}
Write-Ok "Working tree clean"

$Head = git -C $Repo rev-parse HEAD 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Fail "Cannot read HEAD"
    exit $ExitCode
}
$Head = if ($Head -is [array]) { ($Head -join "").Trim() } else { $Head.Trim() }
Write-Ok "HEAD = $Head"

$null = git -C $Repo rev-parse --verify $Base 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Fail "Base commit does not exist: $Base"
    exit $ExitCode
}
Write-Ok "Base commit exists"

if (-not (Test-IsAncestor -Ancestor $Base -Descendant $Head -RepoParam $Repo)) {
    Write-Fail "Base is not an ancestor of HEAD"
    exit $ExitCode
}
Write-Ok "Base is ancestor of HEAD"

if ($PreviousHead) {
    $null = git -C $Repo rev-parse --verify $PreviousHead 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Fail "PreviousHead does not exist: $PreviousHead"
        exit $ExitCode
    }
    if (-not (Test-IsAncestor -Ancestor $PreviousHead -Descendant $Head -RepoParam $Repo)) {
        Write-Fail "PreviousHead is not an ancestor of HEAD"
        exit $ExitCode
    }
    Write-Ok "PreviousHead = $PreviousHead"
}

if (-not (Test-Path -LiteralPath $LogsDirectory -PathType Container)) {
    Write-Fail "Logs directory does not exist: $LogsDirectory"
    exit $ExitCode
}
$logFiles = @(Get-ChildItem -LiteralPath $LogsDirectory -File)
if ($logFiles.Count -eq 0) {
    Write-Fail "Logs directory contains no files: $LogsDirectory"
    exit $ExitCode
}
Write-Ok "Logs directory has $($logFiles.Count) file(s)"

if (-not (Test-Path -LiteralPath $TestSummaryPath -PathType Leaf)) {
    Write-Fail "Test summary does not exist: $TestSummaryPath"
    exit $ExitCode
}
Write-Ok "Test summary found"

$repoRootOuter = git -C $Repo rev-parse --show-toplevel 2>&1
$repoRootOuter = [System.IO.Path]::GetFullPath($repoRootOuter)
if (-not (Test-OutsideRepo -OutputDir $OutputRoot -RepoDirParam $repoRootOuter)) {
    Write-Fail "Output root must be outside the repository."
    exit $ExitCode
}
Write-Ok "Output root is outside repository"

if ($ArtifactPath) {
    if (-not (Test-Path -LiteralPath $ArtifactPath -PathType Leaf)) {
        Write-Fail "Artifact does not exist: $ArtifactPath"
        exit $ExitCode
    }
    $artifactBytes = [System.IO.File]::ReadAllBytes($ArtifactPath)
    if ($artifactBytes.Length -lt 2 -or $artifactBytes[0] -ne 0x4D -or $artifactBytes[1] -ne 0x5A) {
        Write-Fail "Artifact does not have MZ header: $ArtifactPath"
        exit $ExitCode
    }
    Write-Ok "Artifact is valid PE (MZ header)"
}

if ((Test-Path -LiteralPath $ZipPath -PathType Leaf) -and (-not $Force)) {
    Write-Fail "Output ZIP already exists. Use -Force: $ZipPath"
    exit $ExitCode
}

# ---- Step 2: staging + evidence + ZIP (all inside single try/finally) ----

try {
    # Create staging directories
    $Staging = Join-Path $OutputRoot ".quackquota-staging-$SafeName"
    if (Test-Path -LiteralPath $Staging) {
        Remove-Item -LiteralPath $Staging -Recurse -Force
    }
    New-Item -ItemType Directory -Path $Staging -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $Staging "logs") -Force | Out-Null

    Write-Info "Staging at: $Staging"

    # FIX-3A: SelfTestFault "staging-create-fault" — throw after staging created
    if ($SelfTestFault -eq "staging-create-fault") {
        Write-Info "[SelfTestFault] staging-create-fault — throwing after staging creation"
        throw "SelfTestFault: staging-create-fault"
    }

    # ---- Step 3: collect evidence ----

    $branch = Invoke-GitLines -RepoPath $Repo -GitArgs @("branch", "--show-current")
    if ($LASTEXITCODE -ne 0) { $branch = "detached" }
    $branch = $branch.Trim()

    Write-Info "Collecting git evidence on branch: $branch"

    $statusFull = Invoke-GitLines -RepoPath $Repo -GitArgs @("status", "--short", "--branch")
    New-Utf8File -Path (Join-Path $Staging "git-status.txt") -Content $statusFull
    Write-Ok "git-status.txt"

    New-Utf8File -Path (Join-Path $Staging "git-head.txt") -Content $Head
    Write-Ok "git-head.txt"

    $gitLog = Invoke-GitLines -RepoPath $Repo -GitArgs @("log", "--oneline", "${Base}..HEAD")
    New-Utf8File -Path (Join-Path $Staging "git-log.txt") -Content $gitLog
    Write-Ok "git-log.txt"

    $changedFiles = Invoke-GitLines -RepoPath $Repo -GitArgs @("diff", "--name-only", $Base, "HEAD")
    New-Utf8File -Path (Join-Path $Staging "changed-files.txt") -Content $changedFiles
    Write-Ok "changed-files.txt"

    $fullStat = Invoke-GitLines -RepoPath $Repo -GitArgs @("diff", "--stat", $Base, "HEAD")
    New-Utf8File -Path (Join-Path $Staging "full-stat.txt") -Content $fullStat
    Write-Ok "full-stat.txt"

    $fullPatchPath = Join-Path $Staging "full.patch"
    git -C $Repo diff --binary $Base HEAD --output=$fullPatchPath 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "Failed to generate full.patch" }
    Write-Ok "full.patch"

    $headZipPath = Join-Path $Staging "head.zip"
    git -C $Repo archive --format=zip --output=$headZipPath HEAD 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "Failed to generate head.zip" }
    Write-Ok "head.zip"

    if ($PreviousHead) {
        $incStat = Invoke-GitLines -RepoPath $Repo -GitArgs @("diff", "--stat", $PreviousHead, "HEAD")
        New-Utf8File -Path (Join-Path $Staging "incremental-stat.txt") -Content $incStat
        Write-Ok "incremental-stat.txt"

        $incPatchPath = Join-Path $Staging "incremental.patch"
        git -C $Repo diff --binary $PreviousHead HEAD --output=$incPatchPath 2>&1 | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "Failed to generate incremental.patch" }
        Write-Ok "incremental.patch"
    }

    Copy-Item -LiteralPath $TestSummaryPath -Destination (Join-Path $Staging "test-summary.md") -Force
    Write-Ok "test-summary.md"

    $logDestDir = Join-Path $Staging "logs"
    foreach ($lf in $logFiles) {
        Copy-Item -LiteralPath $lf.FullName -Destination (Join-Path $logDestDir $lf.Name) -Force
    }
    Write-Ok "logs/ ($($logFiles.Count) files)"

    # artifact-info.txt
    $artifactInfo = $null
    if ($ArtifactPath) {
        $item = Get-Item -LiteralPath $ArtifactPath
        $artifactBytes = [System.IO.File]::ReadAllBytes($ArtifactPath)
        $headerRaw = [System.BitConverter]::ToString($artifactBytes[0..1]).Replace("-", " ")
        $header = "${headerRaw} (MZ)"
        $artifactSha = Get-SHA256 -Path $ArtifactPath

        $infoContent = @"
Path=$ArtifactPath
FileName=$($item.Name)
Size=$($item.Length)
CreationTime=$($item.CreationTime.ToString("o"))
LastWriteTime=$($item.LastWriteTime.ToString("o"))
SHA256=$artifactSha
FileHeader=$header
"@
        New-Utf8File -Path (Join-Path $Staging "artifact-info.txt") -Content $infoContent
        Write-Ok "artifact-info.txt"

        $artifactInfo = @{
            path            = $ArtifactPath
            file_name       = $item.Name
            size            = $item.Length
            creation_time   = $item.CreationTime.ToString("o")
            last_write_time = $item.LastWriteTime.ToString("o")
            sha256          = $artifactSha
            file_header     = $header
        }
    }

    # ---- Step 4: manifest ----

    $generatedAt = (Get-Date).ToString("o")
    $prevHeadDisplay = if ($PreviousHead) { $PreviousHead } else { "N/A" }
    $hasArtifactText = if ($ArtifactPath) { "Yes" } else { "No" }
    $hasIncrementalText = if ($PreviousHead) { "Yes" } else { "No" }

    $notExecutedDisplay = if ($NotExecuted.Count -gt 0) {
        ($NotExecuted | ForEach-Object { "- $_" }) -join "`r`n"
    } else {
        "(none)"
    }

    $artifactManifestSection = if ($artifactInfo) {
        @"

## Artifact

- **Path**: $($artifactInfo.path)
- **File Name**: $($artifactInfo.file_name)
- **Size**: $($artifactInfo.size) bytes
- **Creation Time**: $($artifactInfo.creation_time)
- **Last Write Time**: $($artifactInfo.last_write_time)
- **SHA-256**: $($artifactInfo.sha256)
- **File Header**: $($artifactInfo.file_header)
"@
    } else { "" }

    $incTableRows = if ($PreviousHead) {
        "| incremental-stat.txt | Diffstat for PreviousHead..HEAD |`r`n| incremental.patch | Binary-safe incremental diff |"
    } else { "" }

    $artifactTableRows = if ($ArtifactPath) {
        "| artifact-info.txt | Built artifact metadata |"
    } else { "" }

    $manifestContent = @"
# QuackQuota Review Pack Manifest

- **Task Name**: $TaskName
- **Repository**: $Repo
- **Branch**: $branch
- **Base**: $Base
- **PreviousHead**: $prevHeadDisplay
- **HEAD**: $Head
- **Generated At**: $generatedAt
- **Log Files**: $($logFiles.Count)
- **Has Artifact**: $hasArtifactText
- **Has Incremental**: $hasIncrementalText

## Not Executed

$notExecutedDisplay
$artifactManifestSection
## Files Included

| File | Description |
|------|-------------|
| manifest.md | This manifest (Markdown) |
| manifest.json | Machine-readable manifest |
| git-status.txt | Full git status at packaging time |
| git-head.txt | HEAD commit SHA |
| git-log.txt | One-line log from Base to HEAD |
| changed-files.txt | Files changed between Base and HEAD |
| full-stat.txt | Diffstat for Base..HEAD |
| full.patch | Binary-safe full diff (Base..HEAD) |
| head.zip | git archive of HEAD |
| test-summary.md | Test summary provided by caller |
| logs/ | Raw test/validation log files |
$incTableRows
$artifactTableRows
"@

    New-Utf8File -Path (Join-Path $Staging "manifest.md") -Content $manifestContent
    Write-Ok "manifest.md"

    $manifestObj = @{
        task_name       = $TaskName
        repo            = $Repo
        branch          = $branch
        base            = $Base
        previous_head   = if ($PreviousHead) { $PreviousHead } else { $null }
        head            = $Head
        generated_at    = $generatedAt
        log_count       = $logFiles.Count
        has_artifact    = [bool]$ArtifactPath
        has_incremental = [bool]$PreviousHead
        not_executed    = @($NotExecuted)
    }
    if ($artifactInfo) {
        $manifestObj.artifact = $artifactInfo
    }

    $manifestJson = $manifestObj | ConvertTo-Json -Depth 5
    New-Utf8File -Path (Join-Path $Staging "manifest.json") -Content $manifestJson
    Write-Ok "manifest.json"

    # ---- Step 5: create ZIP ----

    if (Test-Path -LiteralPath $ZipPath) {
        Remove-Item -LiteralPath $ZipPath -Force
    }

    if (-not (Test-Path -LiteralPath $OutputRoot -PathType Container)) {
        New-Item -ItemType Directory -Path $OutputRoot -Force | Out-Null
    }

    Write-Info "Creating ZIP: $ZipPath"

    Add-Type -AssemblyName System.IO.Compression
    Add-Type -AssemblyName System.IO.Compression.FileSystem

    $zipFs = [System.IO.File]::Create($ZipPath)
    $zipArchive = New-Object System.IO.Compression.ZipArchive($zipFs, [System.IO.Compression.ZipArchiveMode]::Create)

    $allStagingFiles = Get-ChildItem -LiteralPath $Staging -Recurse -File

    $skipEntry = ""
    if ($SelfTestFault -eq "omit-required-entry") {
        $requiredForSkip = @("manifest.md", "manifest.json", "git-status.txt", "git-head.txt",
            "git-log.txt", "changed-files.txt", "full-stat.txt", "full.patch",
            "head.zip", "test-summary.md")
        $skipEntry = $requiredForSkip[0]
        Write-Info "[SelfTestFault] omitting entry: $skipEntry"
    }

    foreach ($file in $allStagingFiles) {
        $relativePath = $file.FullName.Substring($Staging.Length).TrimStart('\', '/')
        $entryName = $relativePath.Replace('\', '/')

        if ($SelfTestFault -eq "omit-required-entry" -and $entryName -eq $skipEntry) {
            continue
        }

        $entry = $zipArchive.CreateEntry($entryName, [System.IO.Compression.CompressionLevel]::Optimal)
        $entryStream = $entry.Open()
        try {
            $fileStream = [System.IO.File]::OpenRead($file.FullName)
            try {
                $fileStream.CopyTo($entryStream)
            } finally {
                $fileStream.Dispose()
            }
        } finally {
            $entryStream.Dispose()
        }
    }

    $zipArchive.Dispose()
    $zipArchive = $null
    $zipFs.Dispose()
    $zipFs = $null

    Write-Ok "ZIP created"

    # ---- Step 6: self-validation ----

    Write-Info "Self-validating ZIP..."

    $validationFailed = $false
    $validationErrors = New-Object System.Collections.Generic.List[string]

    try {
        $verifyFs = [System.IO.File]::OpenRead($ZipPath)
        $verifyArchive = New-Object System.IO.Compression.ZipArchive($verifyFs, [System.IO.Compression.ZipArchiveMode]::Read)

        $entryNames = @($verifyArchive.Entries | ForEach-Object { $_.FullName })

        $backslashEntries = @($entryNames | Where-Object { $_ -match '\\' })
        if ($backslashEntries.Count -gt 0) {
            $validationFailed = $true
            $msg = "ZIP entries contain backslash: $($backslashEntries -join ', ')"
            $validationErrors.Add($msg)
            Write-Fail $msg
        } else {
            Write-Ok "No backslashes in entry names"
        }

        $requiredEntries = @(
            "manifest.md", "manifest.json", "git-status.txt", "git-head.txt",
            "git-log.txt", "changed-files.txt", "full-stat.txt", "full.patch",
            "head.zip", "test-summary.md"
        )
        foreach ($req in $requiredEntries) {
            if ($req -notin $entryNames) {
                $validationFailed = $true
                $msg = "Missing required entry: $req"
                $validationErrors.Add($msg)
                Write-Fail $msg
            }
        }
        if (-not $validationFailed) { Write-Ok "Required entries present" }

        $logEntries = @($entryNames | Where-Object { $_.StartsWith("logs/") })
        if ($logEntries.Count -eq 0) {
            $validationFailed = $true
            $msg = "No logs/ entries found"
            $validationErrors.Add($msg)
            Write-Fail $msg
        } else {
            Write-Ok "logs/ entries present ($($logEntries.Count))"
        }

        $fullPatchEntry = $verifyArchive.GetEntry("full.patch")
        if ($fullPatchEntry -and $fullPatchEntry.Length -gt 0) {
            Write-Ok "full.patch has content ($($fullPatchEntry.Length) bytes)"
        } else {
            $validationFailed = $true
            $msg = "full.patch is empty or missing"
            $validationErrors.Add($msg)
            Write-Fail $msg
        }

        $headZipEntry = $verifyArchive.GetEntry("head.zip")
        if ($headZipEntry -and $headZipEntry.Length -gt 0) {
            try {
                $headZipStream = $headZipEntry.Open()
                $headZipCheck = New-Object System.IO.Compression.ZipArchive($headZipStream, [System.IO.Compression.ZipArchiveMode]::Read)
                Write-Ok "head.zip is a valid ZIP ($($headZipEntry.Length) bytes)"
                $headZipCheck.Dispose()
                $headZipStream.Dispose()
            } catch {
                $validationFailed = $true
                $msg = "head.zip is not a valid ZIP: $_"
                $validationErrors.Add($msg)
                Write-Fail $msg
            }
        } else {
            $validationFailed = $true
            $msg = "head.zip is empty or missing"
            $validationErrors.Add($msg)
            Write-Fail $msg
        }

        $manifestJsonEntry = $verifyArchive.GetEntry("manifest.json")
        if ($manifestJsonEntry) {
            $reader = New-Object System.IO.StreamReader($manifestJsonEntry.Open(), [System.Text.Encoding]::UTF8)
            $manifestJsonContent = $reader.ReadToEnd()
            $reader.Dispose()
            try {
                $parsed = $manifestJsonContent | ConvertFrom-Json
                if ($parsed.repo -ne $Repo) {
                    $validationFailed = $true
                    $msg = "manifest.json repo mismatch"
                    $validationErrors.Add($msg)
                    Write-Fail $msg
                }
                if ($parsed.base -ne $Base) {
                    $validationFailed = $true
                    $msg = "manifest.json base mismatch"
                    $validationErrors.Add($msg)
                    Write-Fail $msg
                }
                if ($parsed.head -ne $Head) {
                    $validationFailed = $true
                    $msg = "manifest.json head mismatch"
                    $validationErrors.Add($msg)
                    Write-Fail $msg
                }
                $expectedNotExec = @($NotExecuted)
                $actualNotExec = @($parsed.not_executed)
                if (($expectedNotExec.Count -ne $actualNotExec.Count) -or
                    (Compare-Object $expectedNotExec $actualNotExec)) {
                    $validationFailed = $true
                    $msg = "manifest.json not_executed mismatch"
                    $validationErrors.Add($msg)
                    Write-Fail $msg
                }
                Write-Ok "manifest.json valid and matches"
            } catch {
                $validationFailed = $true
                $msg = "manifest.json is not valid JSON: $_"
                $validationErrors.Add($msg)
                Write-Fail $msg
            }
        } else {
            $validationFailed = $true
            $msg = "manifest.json missing"
            $validationErrors.Add($msg)
            Write-Fail $msg
        }

        if ($PreviousHead) {
            if ("incremental.patch" -notin $entryNames) {
                $validationFailed = $true
                $msg = "PreviousHead provided but incremental.patch missing"
                $validationErrors.Add($msg)
                Write-Fail $msg
            }
            if ("incremental-stat.txt" -notin $entryNames) {
                $validationFailed = $true
                $msg = "PreviousHead provided but incremental-stat.txt missing"
                $validationErrors.Add($msg)
                Write-Fail $msg
            }
        }

        if ($ArtifactPath) {
            if ("artifact-info.txt" -notin $entryNames) {
                $validationFailed = $true
                $msg = "ArtifactPath provided but artifact-info.txt missing"
                $validationErrors.Add($msg)
                Write-Fail $msg
            }
            if (-not $parsed.artifact) {
                $validationFailed = $true
                $msg = "ArtifactPath provided but manifest.json has no artifact section"
                $validationErrors.Add($msg)
                Write-Fail $msg
            } elseif (-not $parsed.artifact.sha256) {
                $validationFailed = $true
                $msg = "manifest.json artifact section missing sha256"
                $validationErrors.Add($msg)
                Write-Fail $msg
            }
        }

        $verifyArchive.Dispose()
        $verifyArchive = $null
        $verifyFs.Dispose()
        $verifyFs = $null
    } catch {
        $validationFailed = $true
        $msg = "ZIP validation threw exception: $_"
        $validationErrors.Add($msg)
        Write-Fail $msg
    } finally {
        if ($verifyArchive) { try { $verifyArchive.Dispose() } catch {} }
        if ($verifyFs) { try { $verifyFs.Dispose() } catch {} }
    }

    if ($validationFailed) {
        Write-Fail "ZIP validation failed:"
        foreach ($e in $validationErrors) { Write-Host "  - $e" }
        if (Test-Path -LiteralPath $ZipPath) {
            Remove-Item -LiteralPath $ZipPath -Force
            Write-Info "Invalid ZIP deleted"
        }
        exit $ExitCode
    }

    Write-Ok "ZIP self-validation passed"
} finally {
    # ---- Cleanup staging (always, fail-closed) ----
    if ($Staging -and (Test-Path -LiteralPath $Staging)) {
        try {
            Remove-Item -LiteralPath $Staging -Recurse -Force -ErrorAction Stop
        } catch {
            Write-Fail "Staging cleanup failed: $_"
            $script:StagingCleanupFailed = $true
        }
    }
    if ($zipArchive) { try { $zipArchive.Dispose() } catch {} }
    if ($zipFs) { try { $zipFs.Dispose() } catch {} }
    if ($verifyArchive) { try { $verifyArchive.Dispose() } catch {} }
    if ($verifyFs) { try { $verifyFs.Dispose() } catch {} }
}

# ---- Step 7: final output ----

# Fail-closed: if staging cleanup failed, do not report success
if ($script:StagingCleanupFailed) {
    Write-Fail "Staging cleanup failed — task incomplete"
    exit $ExitCode
}

$zipSha = Get-SHA256 -Path $ZipPath

$ExitCode = 0

Write-Host ""
Write-Host "ReviewZip=$ZipPath"
Write-Host "ReviewZipSHA256=$zipSha"
Write-Host "Head=$Head"
Write-Host "ExitCode=$ExitCode"

exit $ExitCode
