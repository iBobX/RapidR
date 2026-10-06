' QFILESTREAM's WriteByte / ReadByte (26 past the end, as RC.EXE gives),
' QMEMORYSTREAM's SetSize (a property in RC.EXE), CopyFrom of all of a
' stream (0: from its start) and of some bytes from its position.
' Expected output: RC.EXE's.
$INCLUDE "RAPIDQ.INC"
P$ = "tests/conformance/.work/stream_bytes.bin"
DIM F AS QFILESTREAM
DIM M AS QMEMORYSTREAM
DIM N AS QMEMORYSTREAM
F.Open(P$, fmCreate)
F.WriteByte(65)
F.WriteByte(66)
F.WriteByte(300)
F.WriteByte(-1)
PRINT "size "; F.Size; " pos "; F.Position
F.Position = 0
PRINT F.ReadByte; " "; F.ReadByte; " "; F.ReadByte; " "; F.ReadByte
PRINT "pos "; F.Position
PRINT "at end "; F.ReadByte; " pos "; F.Position
B = F.ReadByte
PRINT "again "; B; " eof "; F.EOF
M.SetSize = 10
PRINT "setsize "; M.Size; " pos "; M.Position
M.SetSize = 2
PRINT "setsize "; M.Size
M.Size = 0
F.Position = 2
N.CopyFrom(F, 0)
PRINT "all: size "; N.Size; " pos "; N.Position; " from pos "; F.Position
F.Position = 1
N.CopyFrom(F, 2)
PRINT "two: size "; N.Size; " pos "; N.Position; " from pos "; F.Position
N.Position = 0
S$ = N.ReadStr(N.Size)
FOR I = 1 TO LEN(S$): PRINT ASC(MID$(S$, I, 1));: NEXT: PRINT
F.Close
F.Open(P$, fmOpenRead)
M.CopyFrom(F, 3)
PRINT "file: size "; M.Size; " from pos "; F.Position
M.Position = 0
PRINT M.ReadStr(3)
F.Close
KILL P$
