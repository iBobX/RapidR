' The streams' SaveUDTArray / LoadUDTArray, as RapidQ does them (the
' .expected is RC.EXE's output, docs/rapidq-ground-truth.md): one argument,
' a TYPE's array field; every element of it, laid out as RapidQ stores the
' field's type (STRING * n space-padded), at Position, which moves on. A
' field that isn't an array writes nothing. (Not here: loading more than
' the stream holds — RapidQ's "Stream read error", which RapidR reports.)
$INCLUDE "RAPIDQ.INC"
TYPE TQ
  a AS SHORT
  arr(3) AS LONG
  z AS BYTE
END TYPE
DIM p AS TQ
p.a = 1: p.arr(0) = 5: p.arr(1) = 6: p.arr(2) = -7: p.arr(3) = 8: p.z = 9
DIM Mem AS QMEMORYSTREAM
Mem.SaveUDTArray(p.arr)
PRINT "a "; Mem.Size; " "; Mem.Position
Mem.Position = 0
FOR i = 1 TO Mem.Size: PRINT ASC(Mem.ReadStr(1)); " ";: NEXT
PRINT
DIM q AS TQ
q.arr(3) = 99
Mem.Position = 0
Mem.LoadUDTArray(q.arr)
PRINT "b "; Mem.Position; " "; q.a; " "; q.arr(0); " "; q.arr(1); " "; q.arr(2); " "; q.arr(3); " "; q.z
Mem.Close
Mem.SaveUDTArray(p.a)
PRINT "c "; Mem.Size; " "; Mem.Position

TYPE TS
  n AS INTEGER
  s(2) AS STRING * 3
  d(1) AS DOUBLE
END TYPE
DIM r AS TS
r.s(0) = "ab": r.s(1) = "cde": r.s(2) = "f"
r.d(0) = 1.5: r.d(1) = -2
Mem.Close
Mem.SaveUDTArray(r.s)
PRINT "d "; Mem.Size
Mem.SaveUDTArray(r.d)
PRINT "e "; Mem.Size
Mem.Position = 0
FOR i = 1 TO Mem.Size: PRINT ASC(Mem.ReadStr(1)); " ";: NEXT
PRINT
DIM u AS TS
Mem.Position = 0
Mem.LoadUDTArray(u.s)
Mem.LoadUDTArray(u.d)
PRINT "f ["; u.s(0); "] ["; u.s(1); "] ["; u.s(2); "] "; u.d(0); " "; u.d(1); " "; Mem.Position

TYPE TW
  a AS SHORT
  arr(2) AS WORD
END TYPE
DIM w AS TW
w.arr(0) = 1: w.arr(1) = 2: w.arr(2) = 65535
DIM f AS QFILESTREAM
f.Open("tests/conformance/.work/stream_udt_arrays.bin", fmCreate)
f.SaveUDTArray(w.arr)
PRINT "g "; f.Size; " "; f.Position
f.Position = 0
DIM v AS TW
f.LoadUDTArray(v.arr)
PRINT "h "; v.arr(0); " "; v.arr(1); " "; v.arr(2); " "; f.Position
f.Close
