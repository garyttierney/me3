!include MUI2.nsh

!define PRODUCT "me3"
!define PRODUCT_URL "https://github.com/garyttierney/me3"
!ifndef VERSION
!define VERSION unknown
!endif
!define MUI_ABORTWARNING

Unicode true

Name "me3"
OutFile "me3_installer_${VERSION}.exe"

RequestExecutionLevel admin
InstallDir "$PROGRAMFILES\me3"
InstallDirRegKey HKLM "Software\${PRODUCT}" "Install_Dir"

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
    File "target/x86_64-pc-windows-msvc/release/me3-launcher.exe"
    File "target/x86_64-pc-windows-msvc/release/me3_mod_host.dll"

    WriteRegStr HKLM "$UNINSTALL_REG_KEY" "DisplayName" "me3"
    WriteRegStr HKLM "$UNINSTALL_REG_KEY" "UninstallString" '"$INSTDIR\uninstall.exe"'
    WriteRegStr HKLM "$UNINSTALL_REG_KEY" "InstallLocation" "$INSTDIR"
    WriteRegStr HKLM "$UNINSTALL_REG_KEY" "DisplayVersion" "${VERSION}"
    WriteRegDWORD HKLM "$UNINSTALL_REG_KEY" "NoModify" 1
    WriteRegDWORD HKLM "$UNINSTALL_REG_KEY" "NoRepair" 1

    ; Generate an uninstaller executable
    WriteUninstaller "$INSTDIR\uninstall.exe"
SectionEnd

Section "Uninstall"
    Delete "$INSTDIR\me3-launcher.exe"
    Delete "$INSTDIR\me3_host.dll"
    Delete "$INSTDIR\uninstall.exe"

    RMDir "$INSTDIR"

    DeleteRegKey HKLM "$UNINSTALL_REG_KEY"
SectionEnd