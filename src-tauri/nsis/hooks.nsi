; YOLO Voice NSIS installer hooks
; Tauri calls these functions at specific points during install/uninstall.

!macro NSIS_HOOK_PREINSTALL
  ; Nothing needed before install
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; Nothing needed after install
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; Ask user whether to delete their app data (settings, styles, vocabulary, keys)
  MessageBox MB_YESNO|MB_ICONQUESTION \
    "Do you want to delete your YOLO Voice user data?$\r$\n$\r$\nThis includes settings, dictation styles, vocabulary, and API keys.$\r$\n$\r$\nChoose 'No' to keep your data for a future reinstall." \
    IDYES DeleteData IDNO SkipDelete

  DeleteData:
    RMDir /r "$LOCALAPPDATA\com.alish.yolo-voice"
    Goto Done

  SkipDelete:
  Done:
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; Nothing needed after uninstall
!macroend
