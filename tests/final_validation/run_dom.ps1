param(
    [string]$HappyDomPath = (Join-Path $env:TEMP 'mercury4win-reader-tests/node_modules/happy-dom/lib/index.js'),
    [string]$JsdomPath = (Join-Path $env:TEMP 'mercury4win-markdown-tests/node_modules/jsdom/lib/api.js'),
    [string]$SafeNodePath = (Get-Command node -CommandType Application -ErrorAction Stop | Select-Object -First 1).Source,
    [string]$OutputDirectory = (Join-Path $env:TEMP ('mercury4win-dom-validation-' + (Get-Date -Format 'yyyyMMdd-HHmmss')))
)
$ErrorActionPreference = 'Stop'
$validationRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../..')).Path
foreach ($required in @($HappyDomPath, $JsdomPath, $SafeNodePath)) {
    if (-not (Test-Path -LiteralPath $required -PathType Leaf)) { throw "Missing existing test dependency: $required" }
}
New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
$OutputDirectory = (Resolve-Path -LiteralPath $OutputDirectory).Path
$results = [System.Collections.Generic.List[object]]::new()
function Invoke-DomSuite([string]$Name, [string]$Script, [string[]]$Arguments) {
    $log = Join-Path $OutputDirectory "$Name.log"
    & $SafeNodePath $Script @Arguments *> $log
    $code = $LASTEXITCODE
    $content = Get-Content -LiteralPath $log -Encoding UTF8 -Raw
    $count = 0
    foreach ($match in [regex]::Matches($content, '"passed"\s*:\s*(\[.*?\])', 'Singleline')) {
        $count += @($match.Groups[1].Value | ConvertFrom-Json).Count
    }
    foreach ($match in [regex]::Matches($content, '"passed"\s*:\s*(\d+)')) {
        $count += [int]$match.Groups[1].Value
    }
    if ($code -eq 0 -and $count -eq 0) { throw "Suite returned no assertions: $Name ($log)" }
    $results.Add([pscustomobject]@{ suite = $Name; exit_code = $code; passed = $count; log = $log })
    Write-Output "$Name : exit=$code, passed=$count"
    if ($code -ne 0) { Get-Content -LiteralPath $log -Encoding UTF8 -Tail 12 }
}
$fixtureNames = @('M4W_DETAIL_WIRE_PATH', 'M4W_TRANSLATION_FIXTURE', 'M4W_THEME_FIXTURE', 'M4W_USAGE_FIXTURE')
$savedEnvironment = @{}
foreach ($name in $fixtureNames) { $savedEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, 'Process') }
Push-Location -LiteralPath $validationRoot
try {
    $env:M4W_DETAIL_WIRE_PATH = Join-Path $OutputDirectory 'detail.json'
    $env:M4W_TRANSLATION_FIXTURE = Join-Path $OutputDirectory 'translation.json'
    $env:M4W_THEME_FIXTURE = Join-Path $OutputDirectory 'theme.json'
    $env:M4W_USAGE_FIXTURE = Join-Path $OutputDirectory 'usage.json'
    # Exact producers only; these counts are fixture preparation, not added to
    # the complete Rust suite's independent result.
    foreach ($filter in @(
        'detail_serialization_preserves_nested_entry_optional_fields_and_missing_error',
        'hy_mt_serial_context_and_production_document_fixture',
        'styles_fonts_use_independent_cache_without_rebuilding_body_and_emit_fixture',
        'm4w_012_report_history'
    )) {
        $log = Join-Path $OutputDirectory "$filter.log"
        & (Join-Path $validationRoot 'tests/reader_pipeline/run_rust.ps1') -Filter $filter *> $log
        if ($LASTEXITCODE -ne 0) { throw "Rust fixture producer failed: $filter ($log)" }
        if ((Get-Content -LiteralPath $log -Encoding UTF8 -Raw) -notmatch '1 passed; 0 failed') { throw "Expected one fixture producer: $filter" }
    }
    foreach ($name in $fixtureNames) {
        if (-not (Test-Path -LiteralPath ([Environment]::GetEnvironmentVariable($name, 'Process')))) { throw "Fixture not produced: $name" }
    }
    Invoke-DomSuite 'reader_ownership' 'tests/reader_ownership/run_dom.mjs' @($HappyDomPath)
    Invoke-DomSuite 'feed_content' 'tests/reader_ownership/run_dom.mjs' @($HappyDomPath, '../feed_content/browser.tsx')
    Invoke-DomSuite 'reader_pipeline' 'tests/reader_ownership/run_dom.mjs' @($HappyDomPath, '../reader_pipeline/browser.tsx')
    foreach ($suite in @('entry_pagination', 'agent_runtime', 'agent_persistence', 'agent_configuration', 'digest_export', 'tag_system')) {
        Invoke-DomSuite $suite "tests/$suite/run_dom.mjs" @($HappyDomPath)
    }
    Invoke-DomSuite 'entry_filters' 'tests/entry_filters/run_dom.mjs' @($HappyDomPath, $env:M4W_DETAIL_WIRE_PATH)
    Invoke-DomSuite 'translation' 'tests/translation/run_dom.mjs' @($HappyDomPath, $env:M4W_TRANSLATION_FIXTURE)
    Invoke-DomSuite 'usage_lifecycle' 'tests/usage_lifecycle/run_dom.mjs' @($HappyDomPath, $env:M4W_USAGE_FIXTURE)
    Invoke-DomSuite 'reader_theme' 'tests/reader_theme/run_dom.mjs' @($HappyDomPath, $env:M4W_THEME_FIXTURE)
    Invoke-DomSuite 'accessibility' 'tests/accessibility/run_dom.mjs' @($HappyDomPath)
    # DOMPurify's safety assertions require jsdom on a supported Node version.
    Invoke-DomSuite 'markdown_safety' 'tests/markdown_safety/run_dom.mjs' @($JsdomPath)
    $resourceLog = Join-Path $OutputDirectory 'resources.log'
    & $SafeNodePath 'tests/accessibility/check_resources.mjs' *> $resourceLog
    $resourceExit = $LASTEXITCODE
    Get-Content -LiteralPath $resourceLog -Encoding UTF8
    $summary = [pscustomobject]@{
        suites = @($results.ToArray())
        total_passed = ($results | Measure-Object -Property passed -Sum).Sum
        failed_suites = @($results | Where-Object exit_code -NE 0).Count
        resource_exit_code = $resourceExit
    }
    $summary | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $OutputDirectory 'summary.json') -Encoding utf8NoBOM
    Write-Output "DOM assertions passed=$($summary.total_passed); failed suites=$($summary.failed_suites); logs=$OutputDirectory"
    if ($summary.failed_suites -ne 0 -or $resourceExit -ne 0) { exit 1 }
} finally {
    foreach ($name in $fixtureNames) { [Environment]::SetEnvironmentVariable($name, $savedEnvironment[$name], 'Process') }
    Pop-Location
}
