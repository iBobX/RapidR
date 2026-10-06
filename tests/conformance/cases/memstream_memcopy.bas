' QMEMORYSTREAM's MemCopyFrom / MemCopyTo: bytes from memory (VARPTR of
' an array's elements, of a variable, the stream's own Pointer) written at
' the position, and the position's bytes copied to memory; both move the
' position on. Expected output: RC.EXE's.
DIM M AS QMEMORYSTREAM
DIM A(0 TO 3) AS INTEGER
DIM B(0 TO 3) AS INTEGER
A(0) = 11: A(1) = 22: A(2) = 33: A(3) = 44
M.WriteStr("xyz", 3)
PRINT "before size "; M.Size; " pos "; M.Position
M.MemCopyFrom(VARPTR(A(0)), 8)
PRINT "after from size "; M.Size; " pos "; M.Position
M.Position = 0
PRINT "first "; M.ReadStr(3)
PRINT "num "; M.ReadNum(4); " "; M.ReadNum(4)
M.Position = 1
M.MemCopyFrom(VARPTR(A(2)), 4)
PRINT "mid size "; M.Size; " pos "; M.Position
M.Position = 3
M.MemCopyTo(VARPTR(B(1)), 8)
PRINT "to pos "; M.Position
PRINT B(0); B(1); B(2); B(3)
M.Position = 0
M.MemCopyTo(VARPTR(B(0)), 4)
PRINT "to0 pos "; M.Position; " "; B(0)
DIM I AS INTEGER
M.Position = 7
M.MemCopyTo(VARPTR(I), 4)
PRINT "i "; I
I = 1234567
M.Position = 0
M.MemCopyFrom(VARPTR(I), 4)
M.Position = 0
PRINT "back "; M.ReadNum(4)
M.Position = 0
M.MemCopyFrom(M.Pointer + 7, 2)
M.Position = 0
PRINT "self "; M.ReadNum(1); M.ReadNum(1); M.ReadNum(1)
M.MemCopyFrom(VARPTR(I), 0)
M.MemCopyTo(VARPTR(I), 0)
PRINT "zero pos "; M.Position; " i "; I
