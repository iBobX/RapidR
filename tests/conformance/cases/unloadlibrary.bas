' UNLOADLIBRARY(DLLName): removes a DLL the program declared from memory;
' one never declared, or not loaded, does nothing (as RapidQ's manual says),
' and so does it off Windows: no error, no warning. The name is the one the
' DECLARE wrote ("USER32" for LIB "USER32"; "USER32.DLL" does nothing).
' Rapidq2.inc ends its programs with such a call.
$INCLUDE "RAPIDQ.INC"

DECLARE FUNCTION UpdateWindow LIB "USER32" ALIAS "UpdateWindow" (hWnd AS LONG) AS LONG

PRINT "start"
UNLOADLIBRARY("xyz")
UNLOADLIBRARY("USER32")
UNLOADLIBRARY("USER32.DLL")
UNLOADLIBRARY("user32")
UNLOADLIBRARY("")
PRINT "done"
