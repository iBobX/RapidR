' QMEMORYSTREAM's MemCopyFrom / MemCopyTo and SetSize, as RapidQ does them:
' the .expected is RC.EXE's output (docs/rapidq-ground-truth.md). MemCopyFrom
' writes n bytes from the address at Position (a gap past the end zeros,
' nothing before the start), MemCopyTo copies n bytes from Position to the
' address — zeros past the data — and moves Position by n, past the end
' too, back for a negative n. SetSize is Size, only written. RapidR's
' addresses are its own (VARPTR, Pointer): one that isn't the program's is
' reported and copies nothing. (Not here: what RapidQ copies from the
' memory past its buffer, and an array element it never set.)
DIM Mem AS QMEMORYSTREAM
DIM s AS STRING
s = "ABCDEFGHIJ"
Mem.WriteStr("0123456789", 10)
Mem.Position = 3
Mem.MemCopyFrom(VARPTR(s), 4)
PRINT "a "; Mem.Position; " "; Mem.Size
Mem.Position = 0
PRINT "["; Mem.ReadStr(Mem.Size); "]"
Mem.Position = 8
Mem.MemCopyFrom(VARPTR(s) + 2, 5)
PRINT "b "; Mem.Position; " "; Mem.Size
Mem.Position = 0
PRINT "["; Mem.ReadStr(Mem.Size); "]"
DIM t AS STRING
t = "xxxxxxxxxx"
Mem.Position = 2
Mem.MemCopyTo(VARPTR(t), 4)
PRINT "c "; Mem.Position; " ["; t; "]"
DIM a(5) AS LONG
a(0) = 9: a(3) = 7
Mem.Close
Mem.WriteNum(258, 4)
Mem.WriteNum(-1, 4)
Mem.Position = 0
Mem.MemCopyTo(VARPTR(a(1)), 8)
PRINT "d "; Mem.Position; " "; a(0); " "; a(1); " "; a(2); " "; a(3)
a(4) = 77
Mem.Position = 8
Mem.MemCopyFrom(VARPTR(a(4)), 4)
PRINT "e "; Mem.Position; " "; Mem.Size
Mem.Position = 8
PRINT "f "; Mem.ReadNum(4)
Mem.Position = 1
Mem.MemCopyFrom(VARPTR(s), 0)
PRINT "h "; Mem.Position; " "; Mem.Size
Mem.Position = 2
Mem.MemCopyFrom(VARPTR(t), -3)
PRINT "o "; Mem.Position; " "; Mem.Size
DIM d AS DOUBLE
d = 9
Mem.Close
Mem.WriteNum(2.5, 8)
Mem.Position = 0
Mem.MemCopyTo(VARPTR(d), 8)
PRINT "k "; d; " "; Mem.Position
t = "zzzzzzzzzz"
Mem.Close
Mem.WriteStr("0123456789", 10)
Mem.Position = 8
Mem.MemCopyTo(VARPTR(t), 4)
PRINT "g "; Mem.Position; " "; Mem.Size; " "; ASC(MID$(t, 1, 1)); " "; ASC(MID$(t, 2, 1)); " "; ASC(MID$(t, 3, 1)); " "; ASC(MID$(t, 4, 1)); " "; ASC(MID$(t, 5, 1))
Mem.Position = 0
Mem.MemCopyTo(VARPTR(t), 0)
PRINT "z "; Mem.Position
Mem.Position = 2
Mem.MemCopyTo(VARPTR(t), -3)
PRINT "n "; Mem.Position; " "; MID$(t, 5)
PRINT "p "; Mem.Pointer <> 0
Mem.Position = 0
Mem.SetSize = 4
PRINT "s "; Mem.Size; " "; Mem.Position
PRINT "["; Mem.ReadStr(Mem.Size); "]"
Mem.SetSize = 8
PRINT "t "; Mem.Size; " "; Mem.Position
