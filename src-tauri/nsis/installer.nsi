Unicode true
!include "MUI2.nsh"

!ifndef VERSION
  !define VERSION "0.2.2"
!endif
!ifndef PAYLOAD
  !error "PAYLOAD must be defined"
!endif
!ifndef OUTFILE
  !define OUTFILE "BiliCatch_${VERSION}_x64-setup.exe"
!endif
!ifndef ICONFILE
  !define ICONFILE "${__FILEDIR__}\..\icons\icon.ico"
!endif

Name "BiliCatch"
OutFile "${OUTFILE}"
InstallDir "$LOCALAPPDATA\BiliCatch"
InstallDirRegKey HKCU "Software\bilicatch\BiliCatch" "InstallDir"
RequestExecutionLevel user
SetCompressor /SOLID lzma

!define MUI_ICON "${ICONFILE}"
!define MUI_UNICON "${ICONFILE}"
!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\bilicatch.exe"
!define MUI_FINISHPAGE_RUN_TEXT "运行 BiliCatch"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "SimpChinese"
!insertmacro MUI_LANGUAGE "English"

Section "BiliCatch" SecMain
  SectionIn RO
  SetOutPath "$INSTDIR"
  File /r "${PAYLOAD}\*.*"
  CreateDirectory "$INSTDIR\download"
  CreateDirectory "$INSTDIR\models"
  CreateDirectory "$INSTDIR\temp"
  CreateDirectory "$SMPROGRAMS\BiliCatch"
  CreateShortCut "$SMPROGRAMS\BiliCatch\BiliCatch.lnk" "$INSTDIR\bilicatch.exe" "" "$INSTDIR\bilicatch.exe" 0
  WriteUninstaller "$INSTDIR\uninstall.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\BiliCatch" "DisplayName" "BiliCatch"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\BiliCatch" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\BiliCatch" "Publisher" "bilicatch"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\BiliCatch" "DisplayIcon" "$INSTDIR\bilicatch.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\BiliCatch" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\BiliCatch" "UninstallString" "$\"$INSTDIR\uninstall.exe$\""
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\BiliCatch" "QuietUninstallString" "$\"$INSTDIR\uninstall.exe$\" /S"
  WriteRegStr HKCU "Software\bilicatch\BiliCatch" "InstallDir" "$INSTDIR"
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\bilicatch.exe"
  Delete "$INSTDIR\ffmpeg.exe"
  Delete "$INSTDIR\whisper-cli.exe"
  Delete "$INSTDIR\sherpa-onnx-offline.exe"
  Delete "$INSTDIR\*.dll"
  Delete "$INSTDIR\fire-red-asr2-tokens.txt"
  RMDir /r "$INSTDIR\licenses"
  Delete "$SMPROGRAMS\BiliCatch\BiliCatch.lnk"
  RMDir "$SMPROGRAMS\BiliCatch"
  Delete "$DESKTOP\BiliCatch.lnk"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\BiliCatch"
  DeleteRegKey HKCU "Software\bilicatch\BiliCatch"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
SectionEnd
