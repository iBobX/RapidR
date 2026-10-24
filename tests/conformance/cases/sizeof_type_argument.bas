' SIZEOF of a TYPE variable as an argument of a call (a SUB, a FUNCTION, in an
' expression), on winmm's MIDIHDR as a 32-bit program lays it out (16 LONGs,
' 64 bytes) and as 64-bit Windows does (pointers as two LONGs, padding: 120
' bytes; examples/windows/midi_sysex_out.rr). The .expected is RC.EXE's output
' (docs/rapidq-ground-truth.md). A TYPE is packed: a BYTE, a LONG, a SHORT and
' STRING * 5 are 12 bytes.
$APPTYPE CONSOLE
TYPE H32
    lpData AS LONG
    dwBufferLength AS LONG
    dwBytesRecorded AS LONG
    dwUser AS LONG
    dwFlags AS LONG
    lpNext AS LONG
    reserved AS LONG
    dwOffset AS LONG
    r0 AS LONG
    r1 AS LONG
    r2 AS LONG
    r3 AS LONG
    r4 AS LONG
    r5 AS LONG
    r6 AS LONG
    r7 AS LONG
END TYPE
TYPE H64
    lpDataLo AS LONG
    lpDataHi AS LONG
    dwBufferLength AS LONG
    dwBytesRecorded AS LONG
    dwUserLo AS LONG
    dwUserHi AS LONG
    dwFlags AS LONG
    gap1 AS LONG
    lpNextLo AS LONG
    lpNextHi AS LONG
    reservedLo AS LONG
    reservedHi AS LONG
    dwOffset AS LONG
    gap2 AS LONG
    r00 AS LONG
    r01 AS LONG
    r02 AS LONG
    r03 AS LONG
    r04 AS LONG
    r05 AS LONG
    r06 AS LONG
    r07 AS LONG
    r08 AS LONG
    r09 AS LONG
    r10 AS LONG
    r11 AS LONG
    r12 AS LONG
    r13 AS LONG
    r14 AS LONG
    r15 AS LONG
END TYPE
TYPE Mixed
    b AS BYTE
    l AS LONG
    s AS SHORT
    t AS STRING * 5
END TYPE
DECLARE SUB Show (n AS LONG)
DECLARE FUNCTION Twice (n AS LONG) AS LONG
DECLARE SUB Inside
DIM a AS H32
DIM c AS H64
DIM m AS Mixed
PRINT SIZEOF(a); " "; SIZEOF(c); " "; SIZEOF(m)
Show SIZEOF(a)
Show(SIZEOF(c))
PRINT Twice(SIZEOF(a)); " "; Twice(SIZEOF(c) - 56); " "; Twice(SIZEOF(m)) + SIZEOF(a)
Inside

SUB Show (n AS LONG)
    PRINT "Show "; n
END SUB

FUNCTION Twice (n AS LONG) AS LONG
    Twice = n * 2
END FUNCTION

SUB Inside
    ' (a TYPE that is local to a SUB)
    DIM h AS H64
    Show SIZEOF(h)
    PRINT Twice(SIZEOF(h))
END SUB
