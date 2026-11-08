' RWin32.inc as a classic program uses it (no RAPIDQ.INC): the classic
' names and TYPEs, RAPIDQ.INC's Windows numbers and True = 1 as the old
' include gave them, the program's own names winning (a CONST and a
' DECLARE the old file lacked, a variable named like a constant, a variable
' and a TYPE of the same name), the 64-bit and 32-bit layouts.
$INCLUDE "c:\rapidq\include\RWIN32.INC"
CONST WM_MOUSEWHEEL = 522
DECLARE FUNCTION GetTickCount LIB "kernel32" ALIAS "GetTickCount" () AS LONG
INFINITE = 7
DIM r AS RECT, p AS POINTAPI, lf AS LOGFONT
DIM msg AS MSG
PRINT "RECT"; SIZEOF(r); " POINTAPI"; SIZEOF(p); " LOGFONT"; SIZEOF(lf)
PRINT "WM_CLOSE"; WM_CLOSE; " WM_MOUSEWHEEL"; WM_MOUSEWHEEL; " INFINITE"; INFINITE
PRINT "MB_ICONERROR"; MB_ICONERROR; " VK_F1"; VK_F1; " True"; True; " False"; False
PRINT "GENERIC_READ"; GENERIC_READ; " HKEY_LOCAL_MACHINE"; HKEY_LOCAL_MACHINE
r.right = 100
lf.lfFaceName = "Arial"
PRINT r.right; " "; LEFT$(lf.lfFaceName, 5)
$IFDEF WIN64
PRINT "MSG"; SIZEOF(msg) = 48
$ELSE
PRINT "MSG"; SIZEOF(msg) = 28
$ENDIF
