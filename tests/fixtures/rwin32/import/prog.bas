' A classic program that includes its own WIN32API.INC (the import test's).
$APPTYPE CONSOLE
$INCLUDE "include\WIN32API.INC"
DIM r AS RECT
DIM n AS INTEGER
n = 3
PRINT WM_CLOSE; SW_SHOWNORMAL; MB_ICONINFORMATION; MY_FLAG
IF ENVIRON$("OS") = "Windows_NT" THEN
    PRINT GetTickCount > 0, FindWindow("Shell_TrayWnd", "") <> -1, GetWindowRect(0, r)
    PRINT GetCurrentDirectory(0, "")
END IF
