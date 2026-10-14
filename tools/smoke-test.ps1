<#PSScriptInfo
.VERSION 1.0
.GUID 8b4e2c1a-7f3d-4a9e-9c2e-1a2b3c4d5e6f
.AUTHOR RWP
.DESCRIPTION Owner smoke-test kit for real PresentMon flows (import/analyze/report/compare). PowerShell 5.1+ compatible, no admin, no network, no new tools.
#>
<#
.SYNOPSIS
  RWP owner smoke-test kit (BLOCKED_ON_USER helper for real PresentMon evidence).

.DESCRIPTION
  PowerShell 5.1+ compatible. No admin needed. No network access. No new tools:
  only built-in PowerShell cmdlets plus `cargo` and the `rwp` binary built from
  this repo. Steps: `rwp doctor [--presentmon]`, `import-presentmon` (or a note
  that GUI capture was used), `analyze`, `report --format md`, and `compare
  --same-scenario` when two session dirs are given. Every step's command, exit
  code, and output is recorded under OutDir. A redacted `summary.md` is written
  by default (username, user-profile paths, and machine name replaced). When
  the input CSV lives under `crates/**/fixtures` (synthetic), the summary
  header says SYNTHETIC FIXTURE - NOT ACCEPTANCE EVIDENCE.

.PARAMETER PresentMon
  Optional path to PresentMon.exe for `rwp doctor --presentmon`.

.PARAMETER CsvPath
  Optional PresentMon CSV path for import mode (`import-presentmon`).

.PARAMETER ProcessId
  Target PID for import mode. Default 42 (matches the synthetic fixture).

.PARAMETER QpcFrequencyHz
  QPC frequency for import mode. Default 1000000 (matches the synthetic fixture).

.PARAMETER OutDir
  Output directory. Default `smoke-out/` (git-ignored). All step logs live here.

.PARAMETER RwpExe
  Optional path to the rwp binary. Default: `target/debug/rwp.exe`, then
  `target/release/rwp.exe`, then `cargo run --locked -p rwp-cli --`.

.PARAMETER Baseline
  Optional first session dir for `compare --same-scenario`.

.PARAMETER Candidate
  Optional second session dir for `compare --same-scenario`.

.PARAMETER SelfCheck
  Runs the redaction self-test on sample strings and exits (0 pass, 1 fail).
  Proves the default redaction removes username, profile paths, and machine name.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File tools/smoke-test.ps1 -SelfCheck

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File tools/smoke-test.ps1 -CsvPath crates/rwp-cli/tests/fixtures/minimal-presentmon-v2.csv -ProcessId 42 -OutDir smoke-out

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File tools/smoke-test.ps1 -PresentMon C:\Tools\PresentMon.exe -CsvPath C:\Captures\run.csv -ProcessId 1234 -QpcFrequencyHz 10000000 -OutDir smoke-out
#>
[CmdletBinding()]
param(
  [string]$PresentMon = "",
  [string]$CsvPath = "",
  [int]$ProcessId = 42,
  [UInt64]$QpcFrequencyHz = 1000000,
  [string]$OutDir = "smoke-out",
  [string]$RwpExe = "",
  [string]$Baseline = "",
  [string]$Candidate = "",
  [switch]$SelfCheck
)

Set-StrictMode -Version 2.0
$ErrorActionPreference = "Stop"

function Redact-String {
  param([string]$Text)
  if ($null -eq $Text -or $Text -eq "") { return "" }
  $redacted = $Text
  $userName = $env:USERNAME
  $userProfile = $env:USERPROFILE
  $machineName = $env:COMPUTERNAME
  if ($machineName -ne $null -and $machineName -ne "") {
    $redacted = [regex]::Replace($redacted, [regex]::Escape($machineName), "<machine>", [System.Text.RegularExpressions.RegexOptions]::IgnoreCase)
  }
  if ($userProfile -ne $null -and $userProfile -ne "") {
    $redacted = [regex]::Replace($redacted, [regex]::Escape($userProfile), "<profile>", [System.Text.RegularExpressions.RegexOptions]::IgnoreCase)
  }
  if ($userName -ne $null -and $userName -ne "") {
    $redacted = [regex]::Replace($redacted, [regex]::Escape($userName), "<user>", [System.Text.RegularExpressions.RegexOptions]::IgnoreCase)
  }
  $redacted = [regex]::Replace($redacted, "C:\\Users\\[^\\\/\s\""']+", "C:\Users\<user>", [System.Text.RegularExpressions.RegexOptions]::IgnoreCase)
  return $redacted
}

function Test-RedactionSelfCheck {
  $userName = $env:USERNAME
  if ($userName -eq $null -or $userName -eq "") { $userName = "TestUser" }
  $userProfile = $env:USERPROFILE
  if ($userProfile -eq $null -or $userProfile -eq "") { $userProfile = "C:\Users\TestUser" }
  $machineName = $env:COMPUTERNAME
  if ($machineName -eq $null -or $machineName -eq "") { $machineName = "TEST-PC" }
  $samples = @(
    "running as $userName on $machineName",
    "profile at $userProfile\Documents\run.csv",
    "other user path C:\Users\SomeOtherName\Captures\a.csv on $machineName"
  )
  $failed = $false
  foreach ($sample in $samples) {
    $redacted = Redact-String -Text $sample
    if ($redacted.Contains($userName) -and (-not $redacted.Contains("<user>")) -and (-not $redacted.Contains("<profile>"))) {
      Write-Host "SELFCHECK FAIL: username leaked: $redacted"
      $failed = $true
    }
    if ($userProfile -ne "" -and $redacted.Contains($userProfile)) {
      Write-Host "SELFCHECK FAIL: profile path leaked: $redacted"
      $failed = $true
    }
    if ($redacted.Contains($machineName)) {
      Write-Host "SELFCHECK FAIL: machine name leaked: $redacted"
      $failed = $true
    }
    if (-not ($redacted.Contains("<user>") -or $redacted.Contains("<profile>") -or $redacted.Contains("<machine>"))) {
      Write-Host "SELFCHECK FAIL: no placeholder in: $redacted"
      $failed = $true
    }
    Write-Host "SELFCHECK sample: $redacted"
  }
  if ($failed) {
    Write-Host "SELFCHECK RESULT: FAIL"
    return 1
  }
  Write-Host "SELFCHECK RESULT: PASS (username/profile/machine redacted)"
  return 0
}

if ($SelfCheck) {
  $code = Test-RedactionSelfCheck
  exit $code
}

function Resolve-RepoRoot {
  $toolsDir = $PSScriptRoot
  if ($toolsDir -eq $null -or $toolsDir -eq "") { $toolsDir = "tools" }
  $root = Split-Path -Parent $toolsDir
  if ($root -eq $null -or $root -eq "") { $root = "." }
  return $root
}

function Resolve-RwpInvocation {
  param([string]$RepoRoot, [string]$RwpExeParam)
  if ($RwpExeParam -ne "" -and (Test-Path -LiteralPath $RwpExeParam)) {
    return @{ Kind = "binary"; Exe = $RwpExeParam; Prefix = @(); Label = $RwpExeParam }
  }
  $debugExe = Join-Path $RepoRoot "target\debug\rwp.exe"
  if (Test-Path -LiteralPath $debugExe) {
    return @{ Kind = "binary"; Exe = $debugExe; Prefix = @(); Label = $debugExe }
  }
  $releaseExe = Join-Path $RepoRoot "target\release\rwp.exe"
  if (Test-Path -LiteralPath $releaseExe) {
    return @{ Kind = "binary"; Exe = $releaseExe; Prefix = @(); Label = $releaseExe }
  }
  $legacyRelease = Join-Path $RepoRoot "target\release\rwp-cli.exe"
  if (Test-Path -LiteralPath $legacyRelease) {
    return @{ Kind = "binary"; Exe = $legacyRelease; Prefix = @(); Label = $legacyRelease }
  }
  return @{ Kind = "cargo"; Exe = "cargo"; Prefix = @("run", "--locked", "-p", "rwp-cli", "--"); Label = "cargo run --locked -p rwp-cli --" }
}

function Invoke-RwpStep {
  param(
    [string[]]$RwpArgs,
    [string]$StdoutFile,
    [string]$StderrFile,
    [Hashtable]$RwpInvocation,
    [string]$RepoRoot
  )
  $displayArgs = $RwpArgs -join " "
  if ($RwpInvocation.Kind -eq "cargo") {
    $commandLine = "cargo run --locked -p rwp-cli -- $displayArgs"
  } else {
    $commandLine = "$($RwpInvocation.Label) $displayArgs"
  }
  $stdoutPath = $StdoutFile
  $stderrPath = $StderrFile
  $exitCode = 1
  $stdoutText = ""
  $stderrText = ""
  try {
    if ($RwpInvocation.Kind -eq "cargo") {
      $allArgs = $RwpInvocation.Prefix + $RwpArgs
      $proc = Start-Process -FilePath $RwpInvocation.Exe -ArgumentList $allArgs -WorkingDirectory $RepoRoot -NoNewWindow -Wait -PassThru -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath
      $exitCode = $proc.ExitCode
    } else {
      $proc = Start-Process -FilePath $RwpInvocation.Exe -ArgumentList $RwpArgs -WorkingDirectory $RepoRoot -NoNewWindow -Wait -PassThru -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath
      $exitCode = $proc.ExitCode
    }
  } catch {
    $stderrText = $_.Exception.Message
    Set-Content -LiteralPath $stderrPath -Value $stderrText -Encoding UTF8
    $exitCode = 1
  }
  if (Test-Path -LiteralPath $stdoutPath) {
    $stdoutText = Get-Content -LiteralPath $stdoutPath -Raw -Encoding UTF8
    if ($null -eq $stdoutText) { $stdoutText = "" }
  }
  if (Test-Path -LiteralPath $stderrPath) {
    $stderrText = Get-Content -LiteralPath $stderrPath -Raw -Encoding UTF8
    if ($null -eq $stderrText) { $stderrText = "" }
  }
  return @{ Command = $commandLine; ExitCode = $exitCode; Stdout = $stdoutText; Stderr = $stderrText }
}

$repoRoot = Resolve-RepoRoot
if (-not (Test-Path -LiteralPath $repoRoot)) { $repoRoot = "." }

if ($OutDir -eq "" ) { $OutDir = "smoke-out" }
$outFull = $OutDir
if (-not [System.IO.Path]::IsPathRooted($OutDir)) {
  $outFull = Join-Path $repoRoot $OutDir
}
New-Item -ItemType Directory -Force -Path $outFull | Out-Null

$runLog = Join-Path $outFull "run.log"
Set-Content -LiteralPath $runLog -Value "RWP smoke test run log (commands, exit codes, outputs)" -Encoding UTF8

function Write-RunLog {
  param([string]$Line)
  Add-Content -LiteralPath $runLog -Value $Line -Encoding UTF8
}

$rwpInv = Resolve-RwpInvocation -RepoRoot $repoRoot -RwpExeParam $RwpExe
Write-RunLog -Line ("rwp invocation: " + $rwpInv.Label)
Write-RunLog -Line ("repo root: " + $repoRoot)
Write-Host "Using rwp: $($rwpInv.Label)"
Write-Host "OutDir: $outFull"

$doctorStdout = Join-Path $outFull "00-doctor.stdout.txt"
$doctorStderr = Join-Path $outFull "00-doctor.stderr.txt"
$doctorArgs = @("doctor")
if ($PresentMon -ne "") {
  $doctorArgs += @("--presentmon", $PresentMon)
}
$doctorResult = Invoke-RwpStep -RwpArgs $doctorArgs -StdoutFile $doctorStdout -StderrFile $doctorStderr -RwpInvocation $rwpInv -RepoRoot $repoRoot
Write-RunLog -Line ("STEP doctor: " + $doctorResult.Command)
Write-RunLog -Line ("exit: " + $doctorResult.ExitCode)
Write-RunLog -Line ("stdout file: 00-doctor.stdout.txt")
Write-RunLog -Line ("stderr file: 00-doctor.stderr.txt")
Write-Host "doctor exit: $($doctorResult.ExitCode)"

$presentmonSha = "n/a (no --PresentMon given)"
$presentmonVersion = "n/a"
if ($PresentMon -ne "" -and (Test-Path -LiteralPath $PresentMon)) {
  try {
    $hash = Get-FileHash -LiteralPath $PresentMon -Algorithm SHA256
    $presentmonSha = $hash.Hash
  } catch {
    $presentmonSha = "hash failed: $($_.Exception.Message)"
  }
}
try {
  $doctorJson = $doctorResult.Stdout | ConvertFrom-Json
  if ($doctorJson.presentmon -ne $null -and $doctorJson.presentmon.major_version -ne $null) {
    $presentmonVersion = "major_version=$($doctorJson.presentmon.major_version) supported=$($doctorJson.presentmon.supported_version)"
  } elseif ($doctorJson.presentmon -eq $null) {
    $presentmonVersion = "n/a (doctor presentmon:null; pass --PresentMon to validate)"
  }
} catch {
  $presentmonVersion = "unparseable doctor stdout (exit $($doctorResult.ExitCode))"
}

$rwpSha = "via cargo run (binary built from current tree; hash UNVERIFIED until a release binary is used)"
if ($rwpInv.Kind -eq "binary") {
  try {
    $rwpHash = Get-FileHash -LiteralPath $rwpInv.Exe -Algorithm SHA256
    $rwpSha = $rwpHash.Hash + " (" + $rwpInv.Exe + ")"
  } catch {
    $rwpSha = "hash failed: $($_.Exception.Message)"
  }
}

$importedSession = ""
$analyzedSession = ""
if ($CsvPath -ne "") {
  $csvFull = $CsvPath
  if (-not [System.IO.Path]::IsPathRooted($CsvPath)) {
    $csvFull = Join-Path $repoRoot $CsvPath
  }
  if (-not (Test-Path -LiteralPath $csvFull)) {
    Write-RunLog -Line ("STEP import-presentmon: CSV not found: " + $csvFull)
    Write-Host "CSV not found: $csvFull"
    exit 2
  }
  $importedSession = Join-Path $outFull "session-imported"
  if (Test-Path -LiteralPath $importedSession) {
    Remove-Item -LiteralPath $importedSession -Recurse -Force
  }
  $importStdout = Join-Path $outFull "01-import.stdout.txt"
  $importStderr = Join-Path $outFull "01-import.stderr.txt"
  $importArgs = @("import-presentmon", $csvFull, "--process-id", "$ProcessId", "--qpc-frequency-hz", "$QpcFrequencyHz", "--output-dir", $importedSession)
  $importResult = Invoke-RwpStep -RwpArgs $importArgs -StdoutFile $importStdout -StderrFile $importStderr -RwpInvocation $rwpInv -RepoRoot $repoRoot
  Write-RunLog -Line ("STEP import-presentmon: " + $importResult.Command)
  Write-RunLog -Line ("exit: " + $importResult.ExitCode)
  Write-Host "import-presentmon exit: $($importResult.ExitCode)"
  if ($importResult.ExitCode -ne 0) {
    Write-Host "import-presentmon failed; see $importStderr"
    exit $importResult.ExitCode
  }
} else {
  Write-RunLog -Line "STEP import-presentmon: SKIPPED (no -CsvPath; GUI capture was used)"
  Write-Host "No -CsvPath: assuming GUI capture was used (import skipped)."
}

if ($importedSession -ne "" -and (Test-Path -LiteralPath $importedSession)) {
  $analyzedSession = Join-Path $outFull "session-analyzed"
  if (Test-Path -LiteralPath $analyzedSession) {
    Remove-Item -LiteralPath $analyzedSession -Recurse -Force
  }
  $analyzeStdout = Join-Path $outFull "02-analyze.stdout.txt"
  $analyzeStderr = Join-Path $outFull "02-analyze.stderr.txt"
  $analyzeArgs = @("analyze", "--session", $importedSession, "--output-dir", $analyzedSession)
  $analyzeResult = Invoke-RwpStep -RwpArgs $analyzeArgs -StdoutFile $analyzeStdout -StderrFile $analyzeStderr -RwpInvocation $rwpInv -RepoRoot $repoRoot
  Write-RunLog -Line ("STEP analyze: " + $analyzeResult.Command)
  Write-RunLog -Line ("exit: " + $analyzeResult.ExitCode)
  Write-Host "analyze exit: $($analyzeResult.ExitCode)"
  if ($analyzeResult.ExitCode -ne 0) {
    Write-Host "analyze failed; see $analyzeStderr"
    exit $analyzeResult.ExitCode
  }
}

$reportSession = $analyzedSession
if ($reportSession -eq "" -or (-not (Test-Path -LiteralPath $reportSession))) {
  $reportSession = $importedSession
}
$reportText = ""
if ($reportSession -ne "" -and (Test-Path -LiteralPath $reportSession)) {
  $reportStdout = Join-Path $outFull "03-report.stdout.md"
  $reportStderr = Join-Path $outFull "03-report.stderr.txt"
  $reportArgs = @("report", "--session", $reportSession, "--format", "md")
  $reportResult = Invoke-RwpStep -RwpArgs $reportArgs -StdoutFile $reportStdout -StderrFile $reportStderr -RwpInvocation $rwpInv -RepoRoot $repoRoot
  Write-RunLog -Line ("STEP report: " + $reportResult.Command)
  Write-RunLog -Line ("exit: " + $reportResult.ExitCode)
  Write-Host "report exit: $($reportResult.ExitCode)"
  if ($reportResult.ExitCode -ne 0) {
    Write-Host "report failed; see $reportStderr"
    exit $reportResult.ExitCode
  }
  $reportText = Get-Content -LiteralPath $reportStdout -Raw -Encoding UTF8
  if ($null -eq $reportText) { $reportText = "" }
} else {
  Write-RunLog -Line "STEP report: SKIPPED (no session dir available)"
}

$compareText = ""
if ($Baseline -ne "" -and $Candidate -ne "" -and (Test-Path -LiteralPath $Baseline) -and (Test-Path -LiteralPath $Candidate)) {
  $compareStdout = Join-Path $outFull "04-compare.stdout.txt"
  $compareStderr = Join-Path $outFull "04-compare.stderr.txt"
  $compareArgs = @("compare", "--baseline", $Baseline, "--candidate", $Candidate, "--same-scenario")
  $compareResult = Invoke-RwpStep -RwpArgs $compareArgs -StdoutFile $compareStdout -StderrFile $compareStderr -RwpInvocation $rwpInv -RepoRoot $repoRoot
  Write-RunLog -Line ("STEP compare: " + $compareResult.Command)
  Write-RunLog -Line ("exit: " + $compareResult.ExitCode)
  Write-Host "compare exit: $($compareResult.ExitCode)"
  if ($compareResult.ExitCode -ne 0) {
    Write-Host "compare failed; see $compareStderr"
    exit $compareResult.ExitCode
  }
  $compareText = Get-Content -LiteralPath $compareStdout -Raw -Encoding UTF8
  if ($null -eq $compareText) { $compareText = "" }
} else {
  Write-RunLog -Line "STEP compare: SKIPPED (needs -Baseline and -Candidate session dirs)"
}

$isSynthetic = $false
if ($CsvPath -ne "") {
  $csvNorm = $CsvPath.Replace("/", "\")
  if ($csvNorm.ToLower().Contains("\crates\") -and $csvNorm.ToLower().Contains("\fixtures\")) {
    $isSynthetic = $true
  }
  try {
    $resolved = Resolve-Path -LiteralPath $csvFull -ErrorAction SilentlyContinue
    if ($resolved -ne $null) {
      $csvResolved = $resolved.Path
      if ($csvResolved -ne $null -and $csvResolved -ne "") {
        $csvLower = $csvResolved.ToLower()
        if ($csvLower.Contains("\crates\") -and $csvLower.Contains("\fixtures\")) {
          $isSynthetic = $true
        }
      }
    }
  } catch {
  }
}

$summaryPath = Join-Path $outFull "summary.md"
$header = "# RWP SMOKE TEST SUMMARY (redacted)"
if ($isSynthetic) {
  $header = "# SYNTHETIC FIXTURE - NOT ACCEPTANCE EVIDENCE"
}
$lines = @()
$lines += $header
$lines += ""
$lines += "Redacted by default: `<user>`, `<profile>`, `<machine>` replace the Windows username, full user-profile paths, and machine name. Share only this file, never raw logs or unredacted bundles."
if ($isSynthetic) {
  $lines += "Input CSV lives under `crates/**/fixtures` (synthetic). Real-PresentMon acceptance stays BLOCKED_ON_USER."
}
$lines += ""
$lines += "## Environment"
$lines += "- rwp invocation: $($rwpInv.Label)"
$lines += "- rwp SHA-256: $rwpSha"
$lines += "- PresentMon path: $(if ($PresentMon -ne '') { $PresentMon } else { 'n/a (not supplied)' })"
$lines += "- PresentMon SHA-256: $presentmonSha"
$lines += "- PresentMon version (doctor): $presentmonVersion"
$lines += "- CSV input: $(if ($CsvPath -ne '') { $CsvPath } else { 'n/a (GUI capture was used)' })"
$lines += "- ProcessId: $ProcessId; QPC Hz: $QpcFrequencyHz"
$lines += ""
$lines += "## Steps (command, exit code, output file)"
$runLogContent = Get-Content -LiteralPath $runLog -Raw -Encoding UTF8
if ($null -eq $runLogContent) { $runLogContent = "" }
$lines += "``````"
$lines += $runLogContent.Trim()
$lines += "``````"
$lines += ""
if ($reportText -ne "") {
  $lines += "## Report (md, redacted)"
  $lines += "``````markdown"
  $lines += $reportText.Trim()
  $lines += "``````"
  $lines += ""
}
if ($compareText -ne "") {
  $lines += "## Compare --same-scenario (redacted)"
  $lines += "``````"
  $lines += $compareText.Trim()
  $lines += "``````"
  $lines += ""
}
$lines += "## Real-PresentMon acceptance"
$lines += "BLOCKED_ON_USER unless a real PresentMon v2 CSV or live capture was used on reference hardware (see docs/acceptance/smoke-test.md)."
$summaryRaw = $lines -join "`r`n"
$summaryRedacted = Redact-String -Text $summaryRaw
Set-Content -LiteralPath $summaryPath -Value $summaryRedacted -Encoding UTF8
Write-Host "Wrote redacted summary: $summaryPath"
Write-RunLog -Line ("summary: " + $summaryPath + " (redacted)")

exit 0
