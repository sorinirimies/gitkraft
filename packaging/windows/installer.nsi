; GitKraft Windows Installer
; Built with NSIS (Nullsoft Scriptable Install System)

!define PRODUCT_NAME "GitKraft"
!define PRODUCT_VERSION "@VERSION@"
!define PRODUCT_PUBLISHER "Sorin Irimies"
!define PRODUCT_URL "https://github.com/sorinirimies/gitkraft"
!define PRODUCT_UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}"

Name "${PRODUCT_NAME} ${PRODUCT_VERSION}"
OutFile "dist\gitkraft-${PRODUCT_VERSION}-windows-x86_64-setup.exe"
InstallDir "$PROGRAMFILES64\GitKraft"
InstallDirRegKey HKLM "${PRODUCT_UNINST_KEY}" "InstallLocation"
RequestExecutionLevel admin
SetCompressor /SOLID lzma

!include "MUI2.nsh"

; UI
!define MUI_ABORTWARNING
!define MUI_ICON "packaging\windows\gitkraft.ico"
!define MUI_UNICON "packaging\windows\gitkraft.ico"

; Pages
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

; Components
Section "GitKraft GUI" SecGUI
    SectionIn RO
    SetOutPath "$INSTDIR"
    File "target\x86_64-pc-windows-msvc\release\gitkraft.exe"
    CreateDirectory "$SMPROGRAMS\GitKraft"
    CreateShortcut "$SMPROGRAMS\GitKraft\GitKraft.lnk" "$INSTDIR\gitkraft.exe"
    CreateShortcut "$DESKTOP\GitKraft.lnk" "$INSTDIR\gitkraft.exe"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\App Paths\gitkraft.exe" "" "$INSTDIR\gitkraft.exe"
SectionEnd

Section "GitKraft TUI" SecTUI
    SetOutPath "$INSTDIR"
    File "target\x86_64-pc-windows-msvc\release\gitkraft-tui.exe"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\App Paths\gitkraft-tui.exe" "" "$INSTDIR\gitkraft-tui.exe"
SectionEnd

Section "Add to PATH" SecPATH
    EnVar::AddValue "PATH" "$INSTDIR"
SectionEnd

; Descriptions
LangString DESC_SecGUI ${LANG_ENGLISH} "GitKraft desktop GUI — mouse-driven Git IDE"
LangString DESC_SecTUI ${LANG_ENGLISH} "GitKraft TUI — keyboard-driven terminal Git IDE"
LangString DESC_SecPATH ${LANG_ENGLISH} "Add GitKraft to the system PATH"

!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
    !insertmacro MUI_DESCRIPTION_TEXT ${SecGUI} $(DESC_SecGUI)
    !insertmacro MUI_DESCRIPTION_TEXT ${SecTUI} $(DESC_SecTUI)
    !insertmacro MUI_DESCRIPTION_TEXT ${SecPATH} $(DESC_SecPATH)
!insertmacro MUI_FUNCTION_DESCRIPTION_END

Section -PostInstall
    WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "DisplayName" "${PRODUCT_NAME} ${PRODUCT_VERSION}"
    WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "UninstallString" "$INSTDIR\uninstall.exe"
    WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "InstallLocation" "$INSTDIR"
    WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "Publisher" "${PRODUCT_PUBLISHER}"
    WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "URLInfoAbout" "${PRODUCT_URL}"
    WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "DisplayVersion" "${PRODUCT_VERSION}"
    WriteRegDWORD HKLM "${PRODUCT_UNINST_KEY}" "NoModify" 1
    WriteRegDWORD HKLM "${PRODUCT_UNINST_KEY}" "NoRepair" 1
    WriteUninstaller "$INSTDIR\uninstall.exe"
SectionEnd

Section Uninstall
    Delete "$INSTDIR\gitkraft.exe"
    Delete "$INSTDIR\gitkraft-tui.exe"
    Delete "$INSTDIR\uninstall.exe"
    Delete "$SMPROGRAMS\GitKraft\GitKraft.lnk"
    Delete "$DESKTOP\GitKraft.lnk"
    RMDir "$SMPROGRAMS\GitKraft"
    RMDir "$INSTDIR"
    DeleteRegKey HKLM "${PRODUCT_UNINST_KEY}"
    DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\App Paths\gitkraft.exe"
    DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\App Paths\gitkraft-tui.exe"
    EnVar::DeleteValue "PATH" "$INSTDIR"
SectionEnd
