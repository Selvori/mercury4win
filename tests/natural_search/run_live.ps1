param(
    [Parameter(Mandatory)][string]$AppDataDirectory,
    [Parameter(Mandatory)][string]$ReportPath,
    [ValidateSet('basic_zh','combined_en','reading_history','unknown_source','unknown_tag','ambiguous_source','metadata_errors')][string]$Case
)
$ErrorActionPreference = 'Stop'
$liveRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../..')).Path
$liveDirectory = (Resolve-Path -LiteralPath $AppDataDirectory).Path
$liveReport = [System.IO.Path]::GetFullPath($ReportPath)
if (!(Test-Path -LiteralPath (Join-Path $liveDirectory 'mercury.db'))) { throw 'Routing database missing' }
if (Test-Path -LiteralPath $liveReport) { throw 'Refuse to overwrite a report' }
if (!(Test-Path -LiteralPath (Split-Path -Parent $liveReport) -PathType Container)) { throw 'Report parent directory missing' }
# Builds and prepares the existing GNU harness. This first invocation skips paid tests.
& (Join-Path $liveRoot 'tests/reader_pipeline/run_rust.ps1') 'credential_store::live_search_filter_tests'
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$liveArtifacts = @(& cargo test --locked --offline --manifest-path (Join-Path $liveRoot 'src-tauri/Cargo.toml') --lib --no-run --message-format=json | ForEach-Object { $_ | ConvertFrom-Json } | Where-Object { $_.reason -eq 'compiler-artifact' -and $_.profile.test -and $_.executable })
if ($LASTEXITCODE -ne 0 -or $liveArtifacts.Count -ne 1) { throw 'Expected one library test executable' }
$livePrevious = @{}
foreach ($liveVariable in @('PATH','M4W_NL_LIVE_APP_DIR','M4W_NL_LIVE_REPORT','M4W_NL_LIVE_CASE')) { $livePrevious[$liveVariable] = [Environment]::GetEnvironmentVariable($liveVariable,'Process') }
try {
    $env:M4W_NL_LIVE_APP_DIR = $liveDirectory
    $env:M4W_NL_LIVE_REPORT = $liveReport
    if ($Case) { $env:M4W_NL_LIVE_CASE = $Case } else { [Environment]::SetEnvironmentVariable('M4W_NL_LIVE_CASE',$null,'Process') }
    $env:PATH = (Join-Path $liveRoot 'src-tauri/target/debug') + ';' + $livePrevious['PATH']
    & $liveArtifacts[0].executable 'agent::credential_store::live_search_filter_tests::approved_live_natural_search' --exact --ignored --nocapture
    $liveExit = $LASTEXITCODE
} finally {
    foreach ($liveVariable in $livePrevious.Keys) { [Environment]::SetEnvironmentVariable($liveVariable,$livePrevious[$liveVariable],'Process') }
}
exit $liveExit
