param([string]$Filter = 'm4w_008_tests')
$ErrorActionPreference = 'Stop'
$readerRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../..')).Path
$readerManifest = Join-Path $readerRoot 'src-tauri/Cargo.toml'
$readerBuildOutput = & cargo test --locked --manifest-path $readerManifest --lib --no-run --message-format=json
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$readerArtifacts = @($readerBuildOutput | ForEach-Object { $_ | ConvertFrom-Json } | Where-Object {
    $_.reason -eq 'compiler-artifact' -and $_.profile.test -and $_.executable
})
if ($readerArtifacts.Count -ne 1) { throw 'Expected one library test executable.' }
$readerExecutable = $readerArtifacts[0].executable
# GNU library test executables lack the app binary's Common Controls 6
# manifest; TaskDialogIndirect otherwise fails before the harness can start.
$readerAssembly = @'
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity version="1.0.0.0" processorArchitecture="amd64" name="mercury4win.tests" type="win32"/>
  <dependency><dependentAssembly>
    <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0" processorArchitecture="amd64" publicKeyToken="6595b64144ccf1df" language="*"/>
  </dependentAssembly></dependency>
</assembly>
'@
[System.IO.File]::WriteAllText("$readerExecutable.manifest", $readerAssembly, [System.Text.UTF8Encoding]::new($false))
# Refresh the executable timestamp so Windows reloads the sidecar manifest.
(Get-Item -LiteralPath $readerExecutable).LastWriteTime = Get-Date
$readerPreviousPath = $env:PATH
try {
    $env:PATH = (Join-Path $readerRoot 'src-tauri/target/debug') + ';' + $readerPreviousPath
    & $readerExecutable $Filter
    $readerExit = $LASTEXITCODE
} finally { $env:PATH = $readerPreviousPath }
exit $readerExit
