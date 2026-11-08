' dropin_classic.bas: a program written the classic way, with the classic
' names (MessageBox, GetWindowText, SendMessage, FindWindow, sndPlaySound,
' GetTickCount, ShellExecute, RECT and POINTAPI), whose only change was its
' include line: `$INCLUDE "WIN32API.INC"` became the line below. It
' compiles on every system; it calls Windows only on Windows (elsewhere it
' says so and stops before the first call).
'
' It also has names of its own the old include lacked (WM_MOUSEWHEEL,
' a DECLARE of GetTickCount): they win, with no clash.
$APPTYPE CONSOLE
$INCLUDE "RWin32.inc"

CONST WM_MOUSEWHEEL = &H20A
DECLARE FUNCTION GetTickCount LIB "kernel32" ALIAS "GetTickCount" () AS LONG

DIM r AS RECT
DIM p AS POINTAPI
DIM title AS STRING
DIM hWnd AS LONG
DIM t0 AS LONG

PRINT "RECT"; SIZEOF(r); " POINTAPI"; SIZEOF(p); " WM_CLOSE"; WM_CLOSE; " WM_MOUSEWHEEL"; WM_MOUSEWHEEL
PRINT "MB_ICONINFORMATION"; MB_ICONINFORMATION; " SW_SHOWNORMAL"; SW_SHOWNORMAL; " SND_ASYNC"; SND_ASYNC; " True"; True
IF ENVIRON$("OS") <> "Windows_NT" THEN
    PRINT "not Windows: the calls are Windows'"
    END
END IF

t0 = GetTickCount
hWnd = FindWindow("Shell_TrayWnd", "")
IF hWnd = 0 THEN hWnd = FindWindow(0, "")
title = SPACE$(260)
PRINT "GetWindowText"; GetWindowText(hWnd, title, 260) >= 0
PRINT "GetWindowRect"; GetWindowRect(hWnd, r) <> 0; r.right >= r.left
PRINT "GetCursorPos"; GetCursorPos(p) <> 0
PRINT "SendMessage"; SendMessage(hWnd, WM_NULL, 0, 0) = 0
PRINT "sndPlaySound"; sndPlaySound("", SND_ASYNC OR SND_NODEFAULT) >= 0
IF COMMAND$(1) = "show" THEN
    MessageBox 0, "RWin32.inc works", "RapidR", MB_ICONINFORMATION
    ShellExecute 0, "open", "notepad.exe", "", "", SW_SHOWNORMAL
END IF
PRINT "GetTickCount"; GetTickCount - t0 >= 0
