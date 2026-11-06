' How long a DLL call takes (docs/windows-dll-calls.md §7): the C library's
' labs, 200,000 times — with no memory handed out, then with a MIDIHDR TYPE,
' a 1 KB string buffer and a 1,000-LONG array handed out by VARPTR (every
' call syncs them: real memory and the program's view), then the same with
' the header changed before each call; and a wait with those blocks
' (`SLEEP 0`: the runtime's look at real memory, and the copy-back of the
' 1 KB string variable around the statement).
'
'   rapidr run tools/bench/dll_calls.bas      (macOS, Linux, Windows)
$TYPECHECK ON
TYPE MIDIHDR
    lpData AS LONG
    dwBufferLength AS LONG
    dwBytesRecorded AS LONG
    dwUser AS LONG
    dwFlags AS LONG
    lpNext AS LONG
    reserved AS LONG
    dwOffset AS LONG
    dwReserved AS STRING * 64
END TYPE

DECLARE FUNCTION labs_mac LIB "libSystem.B.dylib" ALIAS "labs" (BYVAL n AS LONG) AS LONG
DECLARE FUNCTION labs_linux LIB "libc.so.6" ALIAS "labs" (BYVAL n AS LONG) AS LONG
DECLARE FUNCTION labs_win LIB "msvcrt" ALIAS "labs" (BYVAL n AS LONG) AS LONG
DECLARE FUNCTION Call1 (n AS LONG) AS LONG

CONST N = 200000
DIM i AS LONG, r AS LONG, which AS LONG
DIM t AS DOUBLE
DIM hdr AS MIDIHDR
DIM buf AS STRING
DIM arr(999) AS LONG

FUNCTION Call1 (n AS LONG) AS LONG
    IF which = 1 THEN
        Result = labs_win(n)
    ELSEIF which = 2 THEN
        Result = labs_linux(n)
    ELSE
        Result = labs_mac(n)
    END IF
END FUNCTION

IF ENVIRON$("OS") = "Windows_NT" THEN
    which = 1
ELSEIF DIREXISTS("/proc") THEN
    which = 2
END IF

t = TIMER
FOR i = 1 TO N
    r = Call1(-i)
NEXT i
PRINT "no memory handed out:      "; FORMAT$("%.2f", (TIMER - t) * 1000000 / N); " us a call"

buf = STRING$(1024, 0)
hdr.lpData = VARPTR(buf)
hdr.dwBufferLength = LEN(buf)
r = VARPTR(arr(0))
t = TIMER
FOR i = 1 TO N
    r = Call1(-i)
NEXT i
PRINT "3 blocks (1.2 KB + 4 KB):  "; FORMAT$("%.2f", (TIMER - t) * 1000000 / N); " us a call"

t = TIMER
FOR i = 1 TO N
    hdr.dwUser = i
    r = Call1(-i)
NEXT i
PRINT "the header changed each:   "; FORMAT$("%.2f", (TIMER - t) * 1000000 / N); " us a call"

t = TIMER
FOR i = 1 TO N
    SLEEP 0
NEXT i
PRINT "a wait's look (SLEEP 0):   "; FORMAT$("%.2f", (TIMER - t) * 1000000 / N); " us"
