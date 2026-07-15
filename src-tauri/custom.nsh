; Mercury custom NSIS hooks
; Ensures WebView2Loader.dll is bundled alongside mercury.exe

!macro NSIS_HOOK_PREINSTALL
  File "/oname=$INSTDIR\WebView2Loader.dll" "..\..\WebView2Loader.dll"
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  Delete "$INSTDIR\WebView2Loader.dll"
!macroend
