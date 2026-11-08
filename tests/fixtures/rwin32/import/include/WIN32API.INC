' A small stand-in for a classic WIN32API.INC, written for RapidR's tests:
' the shape such files have (a guard, $DEFINEs, Public Const, Declare
' without ByVal, a TYPE), a handful of entries, one of them with an old
' mistake (an ALIAS kernel32 doesn't have) and one value of its own.
$IFNDEF __WIN32API
$DEFINE __WIN32API
$DEFINE INTEGER SHORT
Public Const WM_CLOSE = &H10
Public Const SW_SHOWNORMAL = 1
Public Const MB_ICONINFORMATION = &H40&
Public Const MY_FLAG = 7
Type RECT
    Left As Long
    Top As Long
    Right As Long
    Bottom As Long
End Type
Declare Function GetTickCount Lib "kernel32" Alias "GetTickCount" () As Long
Declare Function GetCurrentDirectory Lib "kernel32" Alias "GetCurrentDirectory" (ByVal nBufferLength As Long, ByVal lpBuffer As String) As Long
Declare Function FindWindow Lib "user32" Alias "FindWindowA" (ByVal lpClassName As String, ByVal lpWindowName As String) As Long
Declare Function GetWindowRect Lib "user32" Alias "GetWindowRect" (ByVal hwnd As Long, lpRect As RECT) As Long
$ENDIF
