!define ORRERY_ELECTRON_GUID "ed94a9ba-cfb1-5245-8809-b9bd8561d205"
!define ORRERY_ELECTRON_UNINSTALL "Software\Microsoft\Windows\CurrentVersion\Uninstall\${ORRERY_ELECTRON_GUID}"
!define ORRERY_RUN_KEY "Software\Microsoft\Windows\CurrentVersion\Run"

!macro NSIS_HOOK_PREINSTALL
  Var /GLOBAL OrreryHadLogin
  StrCpy $OrreryHadLogin ""
  Push $R0
  Push $R1
  Push $R2
  Push $R3
  Push $R4
  Push $R5
  Push $R6
  Push $R7
  Push $R8
  ReadRegStr $R7 HKCU "${ORRERY_RUN_KEY}" "dev.ccm.monitor"
  ReadRegStr $R0 HKCU "${ORRERY_ELECTRON_UNINSTALL}" "UninstallString"
  ReadRegStr $R1 HKCU "Software\${ORRERY_ELECTRON_GUID}" "InstallLocation"
  StrCpy $R4 $R1 1 -1
  ${If} $R4 == "\"
    StrCpy $R1 $R1 -1
  ${EndIf}
  StrCpy $R6 $INSTDIR
  StrCpy $R4 $R6 1 -1
  ${If} $R4 == "\"
    StrCpy $R6 $R6 -1
  ${EndIf}
  StrCpy $R5 ""
  StrCpy $R4 $R0 1
  ${If} $R4 == '"'
    StrCpy $R0 $R0 "" 1
    StrCpy $R3 0
    ${Do}
      StrCpy $R4 $R0 1 $R3
      ${If} $R4 == '"'
      ${OrIf} $R4 == ""
        ${ExitDo}
      ${EndIf}
      IntOp $R3 $R3 + 1
    ${Loop}
    StrCpy $R5 $R0 $R3
  ${EndIf}
  ${If} $R5 != ""
  ${AndIf} $R1 != ""
  ${AndIf} $R1 != $R6
    StrLen $R8 $R1
    IntOp $R8 $R8 + 1
    StrCpy $R4 $R5 $R8
    ${If} $R4 == "$R1\"
      DetailPrint "Removing the previous Orrery (Electron) from $R1"
      ClearErrors
      StrCpy $R2 ""
      ExecWait '"$R5" /S /KEEP_APP_DATA --updated _?=$R1' $R2
      ${If} $R2 == "0"
        Delete "$R1\Uninstall *.exe"
        RMDir "$R1"
        ${If} $R7 != ""
          StrCpy $OrreryHadLogin "1"
          DeleteRegValue HKCU "${ORRERY_RUN_KEY}" "dev.ccm.monitor"
        ${EndIf}
      ${EndIf}
    ${EndIf}
  ${EndIf}
  Pop $R8
  Pop $R7
  Pop $R6
  Pop $R5
  Pop $R4
  Pop $R3
  Pop $R2
  Pop $R1
  Pop $R0
!macroend

!macro NSIS_HOOK_POSTINSTALL
  Push $R0
  Push $R1
  ${If} $OrreryHadLogin == "1"
    WriteRegStr HKCU "${ORRERY_RUN_KEY}" "${PRODUCTNAME}" '"$INSTDIR\${MAINBINARYNAME}.exe" --hidden'
  ${EndIf}
  ${GetParameters} $R0
  ClearErrors
  ${GetOptions} $R0 "--force-run" $R1
  ${IfNot} ${Errors}
    Exec '"$INSTDIR\${MAINBINARYNAME}.exe"'
  ${EndIf}
  ClearErrors
  Pop $R1
  Pop $R0
!macroend
