!include MUI2.nsh
!include LogicLib.nsh
!include nsDialogs.nsh
!include Integration.nsh

!define PRODUCT "garyttierney\me3"
!define PRODUCT_URL "https://github.com/garyttierney/me3"

!macro APP_ASSOCIATE EXT FILECLASS DESCRIPTION ICON COMMANDTEXT COMMAND
  ; Backup the previously associated file class
  ReadRegStr $R0 SHELL_CONTEXT "Software\Classes\.${EXT}" ""
  WriteRegStr SHELL_CONTEXT "Software\Classes\.${EXT}" "${FILECLASS}_backup" "$R0"

  WriteRegStr SHELL_CONTEXT "Software\Classes\.${EXT}" "" "${FILECLASS}"

  WriteRegStr SHELL_CONTEXT "Software\Classes\${FILECLASS}" "" `${DESCRIPTION}`
  WriteRegStr SHELL_CONTEXT "Software\Classes\${FILECLASS}\DefaultIcon" "" `${ICON}`
  WriteRegStr SHELL_CONTEXT "Software\Classes\${FILECLASS}\shell" "" "open"
  WriteRegStr SHELL_CONTEXT "Software\Classes\${FILECLASS}\shell\open" "" `${COMMANDTEXT}`
  WriteRegStr SHELL_CONTEXT "Software\Classes\${FILECLASS}\shell\open\command" "" `${COMMAND}`
!macroend

!macro APP_ASSOCIATE_EX EXT FILECLASS DESCRIPTION ICON VERB DEFAULTVERB SHELLNEW COMMANDTEXT COMMAND
  ; Backup the previously associated file class
  ReadRegStr $R0 SHELL_CONTEXT "Software\Classes\.${EXT}" ""
  WriteRegStr SHELL_CONTEXT "Software\Classes\.${EXT}" "${FILECLASS}_backup" "$R0"

  WriteRegStr SHELL_CONTEXT "Software\Classes\.${EXT}" "" "${FILECLASS}"
  StrCmp "${SHELLNEW}" "0" +2
  WriteRegStr SHELL_CONTEXT "Software\Classes\.${EXT}\ShellNew" "NullFile" ""

  WriteRegStr SHELL_CONTEXT "Software\Classes\${FILECLASS}" "" `${DESCRIPTION}`
  WriteRegStr SHELL_CONTEXT "Software\Classes\${FILECLASS}\DefaultIcon" "" `${ICON}`
  WriteRegStr SHELL_CONTEXT "Software\Classes\${FILECLASS}\shell" "" `${DEFAULTVERB}`
  WriteRegStr SHELL_CONTEXT "Software\Classes\${FILECLASS}\shell\${VERB}" "" `${COMMANDTEXT}`
  WriteRegStr SHELL_CONTEXT "Software\Classes\${FILECLASS}\shell\${VERB}\command" "" `${COMMAND}`
!macroend

!macro APP_ASSOCIATE_ADDVERB FILECLASS VERB COMMANDTEXT COMMAND
  WriteRegStr SHELL_CONTEXT "Software\Classes\${FILECLASS}\shell\${VERB}" "" `${COMMANDTEXT}`
  WriteRegStr SHELL_CONTEXT "Software\Classes\${FILECLASS}\shell\${VERB}\command" "" `${COMMAND}`
!macroend

!macro APP_ASSOCIATE_REMOVEVERB FILECLASS VERB
  DeleteRegKey SHELL_CONTEXT `Software\Classes\${FILECLASS}\shell\${VERB}`
!macroend


!macro APP_UNASSOCIATE EXT FILECLASS
  ; Backup the previously associated file class
  ReadRegStr $R0 SHELL_CONTEXT "Software\Classes\.${EXT}" `${FILECLASS}_backup`
  WriteRegStr SHELL_CONTEXT "Software\Classes\.${EXT}" "" "$R0"

  DeleteRegKey SHELL_CONTEXT `Software\Classes\${FILECLASS}`
!macroend

!macro APP_ASSOCIATE_GETFILECLASS OUTPUT EXT
  ReadRegStr ${OUTPUT} SHELL_CONTEXT "Software\Classes\.${EXT}" ""
!macroend


; !defines for use with SHChangeNotify
!ifdef SHCNE_ASSOCCHANGED
!undef SHCNE_ASSOCCHANGED
!endif
!define SHCNE_ASSOCCHANGED 0x08000000
!ifdef SHCNF_FLUSH
!undef SHCNF_FLUSH
!endif
!define SHCNF_FLUSH        0x1000

!macro UPDATEFILEASSOC
; Using the system.dll plugin to call the SHChangeNotify Win32 API function so we
; can update the shell.
  System::Call "shell32::SHChangeNotify(i,i,i,i) (${SHCNE_ASSOCCHANGED}, ${SHCNF_FLUSH}, 0, 0)"
!macroend

!ifndef TARGET_DIR
  !define TARGET_DIR "target/x86_64-pc-windows-msvc/release/"
!endif

!ifndef VERSION
  !define VERSION unknown
!endif

!define MUI_ABORTWARNING

Unicode true

Name "me3"

RequestExecutionLevel user

InstallDir "$LOCALAPPDATA\Programs\${PRODUCT}"
InstallDirRegKey HKCU "Software\${PRODUCT}" "Install_Dir"

ShowInstDetails "show"
ShowUninstDetails "show"

Var UNINSTALL_REG_KEY
Var TelemetryEnabled
Var Dialog
Var Label
Var Checkbox
Var Text


Function .onInit
    ; Set the uninstall registry key path here
    StrCpy $UNINSTALL_REG_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\me3"
    StrCpy $TelemetryEnabled "${BST_CHECKED}"
FunctionEnd

Function onFinish
  ExecShell "open" "$LOCALAPPDATA\garyttierney\me3\config\profiles"
FunctionEnd

Page custom nsDialogsPage nsDialogsPageLeave
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_SHOWREADME "https://me3.readthedocs.io/"
!define MUI_FINISHPAGE_RUN
!define MUI_FINISHPAGE_RUN_FUNCTION onFinish
!define MUI_FINISHPAGE_RUN_TEXT "Open the mod profile folder?"

!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE English

Function nsDialogsPage
  !insertmacro MUI_HEADER_TEXT "me3 Configuration" "Configure me3 system-wide settings"

	nsDialogs::Create 1018
	Pop $Dialog

	${If} $Dialog == error
		Abort
	${EndIf}

	${NSD_CreateCheckbox} 0 30u 100% 10u "Share crash reports with me3 developers?"
	Pop $Checkbox

	${NSD_CreateLabel} 0 0 100% 64u "me3 will upload crash reports to Sentry.io to alert the developers of frequent issues and help with triaging bug reports"
	Pop $Label

	${If} $TelemetryEnabled == ${BST_CHECKED}
		${NSD_Check} $Checkbox
	${EndIf}

	nsDialogs::Show
FunctionEnd

Function nsDialogsPageLeave
	${NSD_GetState} $Checkbox $TelemetryEnabled
FunctionEnd

; Installer Section
Section "Main Application" SEC01
    SectionIn RO
    SetShellVarContext current

    CreateDirectory "$INSTDIR\config"
    CreateDirectory "$INSTDIR\bin"

    SetOutPath "$INSTDIR"
    File /oname=bin\me3.exe "${TARGET_DIR}me3.exe"
    File /oname=bin\me3-launcher.exe "${TARGET_DIR}me3-launcher.exe"
    File /oname=bin\me3_mod_host.dll "${TARGET_DIR}me3_mod_host.dll"
    File /oname=README.txt "INSTALLER_README.txt"
    File "LICENSE-APACHE"
    File "LICENSE-MIT"
    File "CHANGELOG.md"

    WriteRegStr HKCU "$UNINSTALL_REG_KEY" "DisplayName" "me3"
    WriteRegStr HKCU "$UNINSTALL_REG_KEY" "UninstallString" '"$INSTDIR\uninstall.exe"'
    WriteRegStr HKCU "$UNINSTALL_REG_KEY" "InstallLocation" "$INSTDIR"
    WriteRegStr HKCU "$UNINSTALL_REG_KEY" "DisplayVersion" "${VERSION}"
    WriteRegDWORD HKCU "$UNINSTALL_REG_KEY" "NoModify" 1
    WriteRegDWORD HKCU "$UNINSTALL_REG_KEY" "NoRepair" 1

    WriteRegStr HKCU "Software\${PRODUCT}" "Install_Dir" $INSTDIR
    nsExec::Exec '"$INSTDIR\bin\me3.exe" add-to-path'
    nsExec::Exec '"$INSTDIR\bin\me3.exe" profile create -g er eldenring-default'
    nsExec::Exec '"$INSTDIR\bin\me3.exe" profile create -g nr nightreign-default'
    ; Generate an uninstaller executable
    WriteUninstaller "$INSTDIR\uninstall.exe"

    !insertmacro APP_ASSOCIATE "me3" "me3.mod-profile" "me3 mod profile" \
      "$INSTDIR\bin\me3.exe,0" "Open with me3" "$INSTDIR\bin\me3.exe launch --auto-detect -p $\"%1$\""

    IfFileExists "$INSTDIR\config\me3.toml" file_found file_not_found
file_found:
    goto end
file_not_found:
    File /oname=config\me3.toml "support/config-dist.toml"

    ${If} $TelemetryEnabled == ${BST_CHECKED}
      WriteINIStr "config\me3.toml" "me3" "crash_telemetry" "true"
    ${EndIf}
end:
SectionEnd

Section "Uninstall"
    Delete "$INSTDIR\bin\me3-launcher.exe"
    Delete "$INSTDIR\bin\me3_host.dll"
    Delete "$INSTDIR\bin\me3.exe"
    Delete "$INSTDIR\uninstall.exe"
    Delete "$INSTDIR\LICENSE-APACHE"
    Delete "$INSTDIR\LICENSE-MIT"
    Delete "$INSTDIR\CHANGELOG.md"
    Delete "$INSTDIR\README.txt"
    RMDir "$INSTDIR\bin"

    DeleteRegKey HKLM "$UNINSTALL_REG_KEY"
SectionEnd