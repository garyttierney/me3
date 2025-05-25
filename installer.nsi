!include MUI2.nsh

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
OutFile "me3_installer_${VERSION}.exe"

RequestExecutionLevel user

InstallDir "$LOCALAPPDATA\Programs\me3"
InstallDirRegKey HKCU "Software\${PRODUCT}" "Install_Dir"

ShowInstDetails "show"
ShowUninstDetails "show"

Var UNINSTALL_REG_KEY

!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

Function .onInit
    ; Set the uninstall registry key path here
    StrCpy $UNINSTALL_REG_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\me3"
FunctionEnd

; Installer Section
Section "Main Application" SEC01
    SectionIn RO

    SetOutPath "$INSTDIR"
    File "${TARGET_DIR}me3-launcher.exe"
    File "${TARGET_DIR}me3_launcher.pdb"
    File "${TARGET_DIR}me3_mod_host.dll"
    File "${TARGET_DIR}me3_mod_host.pdb"
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
SectionEnd

Section "Uninstall"
    Delete "$INSTDIR\me3-launcher.exe"
    Delete "$INSTDIR\me3_host.dll"
    Delete "$INSTDIR\uninstall.exe"
    Delete "$INSTDIR\LICENSE-APACHE"
    Delete "$INSTDIR\LICENSE-MIT"
    Delete "$INSTDIR\CHANGELOG.md"
    Delete "$INSTDIR\*.pdb"

    RMDir "$INSTDIR"

    DeleteRegKey HKLM "$UNINSTALL_REG_KEY"
SectionEnd