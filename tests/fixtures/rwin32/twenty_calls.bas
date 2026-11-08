' twenty_calls.bas: twenty different Windows calls through RWin32.inc —
' what a program pays for the include is what it uses (a test times it).
$APPTYPE CONSOLE
$INCLUDE "RWin32.inc"
DIM r AS RECT, p AS POINTAPI, t AS SYSTEMTIME, m AS MEMORYSTATUSEX, s AS STRING
DIM lf AS LOGFONT, nid AS NOTIFYICONDATA, ofn AS OPENFILENAME, msg AS MSG
s = SPACE$(260)
IF ENVIRON$("OS") <> "Windows_NT" THEN PRINT "compiled": END
PRINT GetTickCount
PRINT GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)
PRINT GetWindowRect(GetDesktopWindow, r), r.right
PRINT GetCursorPos(p)
GetLocalTime t
PRINT t.wYear
m.dwLength = SIZEOF(m)
PRINT GlobalMemoryStatusEx(m)
PRINT GetWindowsDirectory(s, 260)
PRINT GetComputerName(s, 260)
PRINT GetCurrentProcessId, GetCurrentThreadId
PRINT GetForegroundWindow <> -1
PRINT FindWindow("Shell_TrayWnd", "") <> -1
PRINT IsWindowVisible(GetDesktopWindow)
PRINT GetKeyState(VK_SHIFT) >= -32768
PRINT GetSysColor(COLOR_WINDOW) >= 0
PRINT GetDoubleClickTime > 0
PRINT GetACP > 0
PRINT GetLastError >= 0
PRINT PeekMessage(msg, 0, 0, 0, PM_NOREMOVE) >= 0
PRINT lstrlen("abc")
PRINT SIZEOF(lf), SIZEOF(nid), SIZEOF(ofn)
