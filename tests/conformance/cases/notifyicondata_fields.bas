' QNOTIFYICONDATA as RapidQ's compiler (RC.EXE) has it: cbSize 88, uID the
' program's instance handle (4194304), the rest 0; numbers cut to 32-bit
' integers, a string stored into one is 0; szTip at most 64 characters, up
' to a CHR$(0), a number stored into it ""; SIZEOF 24 (checked against
' RC.EXE: docs/rapidq-ground-truth.md).
$APPTYPE CONSOLE
DIM n AS QNOTIFYICONDATA
PRINT n.cbSize
PRINT n.hWnd; " "; n.uID; " "; n.uFlags; " "; n.uCallBackMessage; " "; n.hIcon
PRINT "["; n.szTip; "]"
n.hWnd = 12: n.uID = 3: n.uFlags = 7: n.uCallbackMessage = 1024: n.hIcon = 55
PRINT n.hWnd; " "; n.uID; " "; n.uFlags; " "; n.uCallBackMessage; " "; n.hIcon
n.szTip = "Hello tray"
PRINT "["; n.szTip; "]"
n.szTip = STRING$(100, "x")
PRINT LEN(n.szTip)
n.szTip = "a" + CHR$(0) + "b"
PRINT LEN(n.szTip)
n.szTip = 42
PRINT "["; n.szTip; "]"
n.hIcon = "77"
PRINT n.hIcon
n.uFlags = 3.6
PRINT n.uFlags
n.hWnd = -5
PRINT n.hWnd
PRINT SIZEOF(n)
