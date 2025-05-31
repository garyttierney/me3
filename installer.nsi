!include MUI2.nsh
!include LogicLib.nsh
!include nsDialogs.nsh

!define PRODUCT "me3"
!define PRODUCT_URL "https://github.com/garyttierney/me3"

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

InstallDir "$LOCALAPPDATA\Programs\me3"
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

Page custom nsDialogsPage nsDialogsPageLeave
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_SHOWREADME "https://me3.readthedocs.io/"
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

    ; Generate an uninstaller executable
    WriteUninstaller "$INSTDIR\uninstall.exe"

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
    Delete "$INSTDIR\uninstall.exe"
    Delete "$INSTDIR\LICENSE-APACHE"
    Delete "$INSTDIR\LICENSE-MIT"
    Delete "$INSTDIR\CHANGELOG.md"
    Delete "$INSTDIR\*.pdb"

    RMDir "$INSTDIR"

    DeleteRegKey HKLM "$UNINSTALL_REG_KEY"
SectionEnd