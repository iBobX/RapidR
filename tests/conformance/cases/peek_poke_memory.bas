' PEEK / POKE on the program's own memory (VARPTR addresses): RapidR's
' addition to RapidQ's console PEEK / POKE, memory-safe, the same on every
' runtime (docs/windows-dll-calls.md §2)
DIM n AS LONG
n = &H04030201
p = VARPTR(n)
PRINT PEEK(p); " "; PEEK(p + 1); " "; PEEK(p + 2); " "; PEEK(p + 3)
POKE p + 1, &HFF
PRINT HEX$(n)
DIM s AS STRING
s = "Hello"
q = VARPTR(s)
PRINT PEEK(q); " "; CHR$(PEEK(q + 4))
POKE q, ASC("J")
PRINT s
DIM a(0 TO 3) AS INTEGER
a(2) = 258
r = VARPTR(a(2))
PRINT PEEK(r); " "; PEEK(r + 1)
POKE r, 3
PRINT a(2)
TYPE TPoint
  x AS LONG
  y AS LONG
END TYPE
DIM pt AS TPoint
pt.y = 7
PRINT PEEK(VARPTR(pt) + 4)
POKE VARPTR(pt), 9
PRINT pt.x
DIM Mem AS QMEMORYSTREAM
Mem.WriteStr("abc")
POKE Mem.Pointer + 1, ASC("X")
Mem.Position = 0
PRINT Mem.ReadStr(3)
