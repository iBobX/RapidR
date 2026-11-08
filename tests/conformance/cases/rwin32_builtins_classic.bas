' skip-on: win32 — there the call succeeds
' RWin32.inc in a classic program: a name RapidR has a built-in of (Sleep)
' is the DLL's, as the classic WIN32API.INC gave it (here kernel32's Sleep,
' in milliseconds), so off Windows the call stops with the Windows-only
' error. A RapidR program keeps RapidR's own SLEEP: rwin32_builtins_rapidr.rr.
$INCLUDE "RWin32.inc"
PRINT "before"
Sleep 0
PRINT "not reached"
