' skip-on: win32 — there kernel32's RtlMoveMemory copies (docs/windows-dll-calls.md §1)
' A program that DECLAREs RtlMoveMemory itself calls that DLL routine as
' declared (here BYVAL addresses, as RapidQ programs do), not RapidR's own
' RTLMOVEMEMORY: on Windows the copy happens, elsewhere the call stops with
' the error naming the function, never a silent wrong copy.
DECLARE SUB RtlMoveMemory LIB "kernel32" (BYVAL Dest AS LONG, BYVAL Src AS LONG, BYVAL n AS LONG)
DIM a AS LONG, b AS LONG
b = 1234
PRINT "before"
RtlMoveMemory VARPTR(a), VARPTR(b), 4
PRINT a
