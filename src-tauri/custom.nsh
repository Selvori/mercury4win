; Mercury custom NSIS hooks
; Ensures WebView2Loader.dll is bundled alongside mercury.exe

!macro NSIS_HOOK_PREINSTALL
  ; NSIS resolves relative paths from the generated installer.nsi location
  ; (target/release/nsis/x64/). WebView2Loader.dll lives two directories up
  ; in target/release/.  If the file does not exist on the build host,
  ; skip it so CI still produces the installer.
  !if /FileExists "..\..\WebView2Loader.dll"
    File "/oname=$INSTDIR\WebView2Loader.dll" "..\..\WebView2Loader.dll"
  !endif
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  Delete "$INSTDIR\WebView2Loader.dll"
!macroend
