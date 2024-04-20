OutFile "jikken-v0.7.1-x86_64-windows.exe"
InstallDir $PROFILE\.jk

!include "LogicLib.nsh"
!include "WinMessages.nsh"

!define SHCNE_ASSOCCHANGED 0x08000000
!define SHCNF_IDLIST 0

Function RefreshShellIcons
  ; By jerome tremblay - april 2003
  System::Call 'shell32.dll::SHChangeNotify(i, i, i, i) v \
  (${SHCNE_ASSOCCHANGED}, ${SHCNF_IDLIST}, 0, 0)'
FunctionEnd

Section

SetOutPath $INSTDIR
File target\release\jk.exe
WriteUninstaller $INSTDIR\uninstaller.exe
Push "$INSTDIR"
Call AddToPath


WriteRegStr HKCR ".jkt" "" "Jikken Test"
WriteRegStr HKCR "Jikken Test" "" "Jikken Test"
FILE favicon.ico
WriteRegStr HKCR "Jikken Test\DefaultIcon" "" "$INSTDIR\favicon.ico,0"
Call RefreshShellIcons

SectionEnd

Section "Uninstall"
DeleteRegKey HKCR ".jkt"
DeleteRegKey HKCR "Jikken Test"
Delete $INSTDIR\jk.exe
Delete $INSTDIR\uninstaller.exe
RMDir $INSTDIR
Push "$INSTDIR"
Call un.RemoveFromPath

SectionEnd

!define StrStr "!insertmacro StrStr"
!macro StrStr ResultVar String SubString
  Push `${String}`
  Push `${SubString}`
  Call StrStr
  Pop `${ResultVar}`
!macroend

!define un.StrStr "!insertmacro un.StrStr"
!macro un.StrStr ResultVar String SubString
  Push `${String}`
  Push `${SubString}`
  Call un.StrStr
  Pop `${ResultVar}`
!macroend
 
Function StrStr
/*After this point:
  ------------------------------------------
  $R0 = SubString (input)
  $R1 = String (input)
  $R2 = SubStringLen (temp)
  $R3 = StrLen (temp)
  $R4 = StartCharPos (temp)
  $R5 = TempStr (temp)*/
 
  ;Get input from user
  Exch $R0
  Exch
  Exch $R1
  Push $R2
  Push $R3
  Push $R4
  Push $R5
 
  ;Get "String" and "SubString" length
  StrLen $R2 $R0
  StrLen $R3 $R1
  ;Start "StartCharPos" counter
  StrCpy $R4 0
 
  ;Loop until "SubString" is found or "String" reaches its end
  ${Do}
    ;Remove everything before and after the searched part ("TempStr")
    StrCpy $R5 $R1 $R2 $R4
 
    ;Compare "TempStr" with "SubString"
    ${IfThen} $R5 == $R0 ${|} ${ExitDo} ${|}
    ;If not "SubString", this could be "String"'s end
    ${IfThen} $R4 >= $R3 ${|} ${ExitDo} ${|}
    ;If not, continue the loop
    IntOp $R4 $R4 + 1
  ${Loop}
 
/*After this point:
  ------------------------------------------
  $R0 = ResultVar (output)*/
 
  ;Remove part before "SubString" on "String" (if there has one)
  StrCpy $R0 $R1 `` $R4
 
  ;Return output to user
  Pop $R5
  Pop $R4
  Pop $R3
  Pop $R2
  Pop $R1
  Exch $R0
FunctionEnd

Function un.StrStr
/*After this point:
  ------------------------------------------
  $R0 = SubString (input)
  $R1 = String (input)
  $R2 = SubStringLen (temp)
  $R3 = StrLen (temp)
  $R4 = StartCharPos (temp)
  $R5 = TempStr (temp)*/
 
  ;Get input from user
  Exch $R0
  Exch
  Exch $R1
  Push $R2
  Push $R3
  Push $R4
  Push $R5
 
  ;Get "String" and "SubString" length
  StrLen $R2 $R0
  StrLen $R3 $R1
  ;Start "StartCharPos" counter
  StrCpy $R4 0
 
  ;Loop until "SubString" is found or "String" reaches its end
  ${Do}
    ;Remove everything before and after the searched part ("TempStr")
    StrCpy $R5 $R1 $R2 $R4
 
    ;Compare "TempStr" with "SubString"
    ${IfThen} $R5 == $R0 ${|} ${ExitDo} ${|}
    ;If not "SubString", this could be "String"'s end
    ${IfThen} $R4 >= $R3 ${|} ${ExitDo} ${|}
    ;If not, continue the loop
    IntOp $R4 $R4 + 1
  ${Loop}
 
/*After this point:
  ------------------------------------------
  $R0 = ResultVar (output)*/
 
  ;Remove part before "SubString" on "String" (if there has one)
  StrCpy $R0 $R1 `` $R4
 
  ;Return output to user
  Pop $R5
  Pop $R4
  Pop $R3
  Pop $R2
  Pop $R1
  Exch $R0
FunctionEnd

; Registry Entry for environment
; All users (Warning: length often exceeds 1024 if many packages are installed):
;!define Environ 'HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment"'
; Current user only:
!define Environ 'HKCU "Environment"'
 
; AddToPath - Appends dir to PATH
;   (does not work on Win9x/ME)
;
; Usage:
;   Push "dir"
;   Call AddToPath
 
Function AddToPath
  Exch $0
  Push $1
  Push $2
  Push $3
  Push $4
 
  ; NSIS ReadRegStr returns empty string on string overflow
  ; Native calls are used here to check actual length of PATH
 
  ; $4 = RegOpenKey(HKEY_CURRENT_USER, "Environment", &$3)
  System::Call "advapi32::RegOpenKey(i 0x80000001, t'Environment', *i.r3) i.r4"
  IntCmp $4 0 0 done done
  ; $4 = RegQueryValueEx($3, "PATH", (DWORD*)0, (DWORD*)0, &$1, ($2=NSIS_MAX_STRLEN, &$2))
  ; RegCloseKey($3)
  System::Call "advapi32::RegQueryValueEx(i $3, t'PATH', i 0, i 0, t.r1, *i ${NSIS_MAX_STRLEN} r2) i.r4"
  System::Call "advapi32::RegCloseKey(i $3)"
 
  ${If} $4 = 234 ; ERROR_MORE_DATA
    DetailPrint "AddToPath: original length $2 > ${NSIS_MAX_STRLEN}"
    MessageBox MB_OK "PATH not updated, original length $2 > ${NSIS_MAX_STRLEN}" /SD IDOK
    Goto done
  ${EndIf}
 
  ${If} $4 <> 0 ; NO_ERROR
    ${If} $4 <> 2 ; ERROR_FILE_NOT_FOUND
      DetailPrint "AddToPath: unexpected error code $4"
      Goto done
    ${EndIf}
    StrCpy $1 ""
  ${EndIf}
 
  ; Check if already in PATH
  Push "$1;"
  Push "$0;"
  Call StrStr
  Pop $2
  StrCmp $2 "" 0 done
  Push "$1;"
  Push "$0\;"
  Call StrStr
  Pop $2
  StrCmp $2 "" 0 done
 
  ; Prevent NSIS string overflow
  StrLen $2 $0
  StrLen $3 $1
  IntOp $2 $2 + $3
  IntOp $2 $2 + 2 ; $2 = strlen(dir) + strlen(PATH) + sizeof(";")
  ${If} $2 > ${NSIS_MAX_STRLEN}
    DetailPrint "AddToPath: new length $2 > ${NSIS_MAX_STRLEN}"
    MessageBox MB_OK "PATH not updated, new length $2 > ${NSIS_MAX_STRLEN}." /SD IDOK
    Goto done
  ${EndIf}
 
  ; Append dir to PATH
  DetailPrint "Add to PATH: $0"
  StrCpy $2 $1 1 -1
  ${If} $2 == ";"
    StrCpy $1 $1 -1 ; remove trailing ';'
  ${EndIf}
  ${If} $1 != "" ; no leading ';'
    StrCpy $0 "$1;$0"
  ${EndIf}
  WriteRegExpandStr ${Environ} "PATH" $0
  SendMessage ${HWND_BROADCAST} ${WM_WININICHANGE} 0 "STR:Environment" /TIMEOUT=5000
 
done:
  Pop $4
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

Function un.RemoveFromPath
  Exch $0
  Push $1
  Push $2
  Push $3
  Push $4
  Push $5
  Push $6

  ReadRegStr $1 ${Environ} "PATH"
  StrCpy $5 $1 1 -1
  ${If} $5 != ";"
    StrCpy $1 "$1;" ; ensure trailing ';'
  ${EndIf}
  Push $1
  Push "$0;"
  Call un.StrStr
  Pop $2 ; pos of our dir
  StrCmp $2 "" done

  DetailPrint "Remove from PATH: $0"
  StrLen $3 "$0;"
  StrLen $4 $2
  StrCpy $5 $1 -$4 ; $5 is now the part before the path to remove
  StrCpy $6 $2 "" $3 ; $6 is now the part after the path to remove
  StrCpy $3 "$5$6"
  StrCpy $5 $3 1 -1
  ${If} $5 == ";"
    StrCpy $3 $3 -1 ; remove trailing ';'
  ${EndIf}
  WriteRegExpandStr ${Environ} "PATH" $3
  SendMessage ${HWND_BROADCAST} ${WM_WININICHANGE} 0 "STR:Environment" /TIMEOUT=5000

done:
  Pop $6
  Pop $5
  Pop $4
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd
