' Streams read and written outside their data, as RapidQ's: the .expected is
' RC.EXE's output (docs/rapidq-ground-truth.md). Position may be set past
' the end (and before the start on a memory stream; a file's stays); reads
' there get no bytes and leave Position; ReadStr(n) and Read(S$) are always
' n characters, spaces where there were no bytes; ReadNum fills what it
' can; a write past the end leaves zeros in the gap, one before the start
' writes nothing; a Size that leaves Position past the new end moves it to
' the old end. (Not here: what RapidQ reads from memory it never wrote — a
' ReadNum with no bytes at all, a Size grown back over old bytes — and
' ReadLine before the start, which hangs RapidQ.)
$INCLUDE "RAPIDQ.INC"
DIM M AS QMEMORYSTREAM
DIM S AS STRING
DIM P AS LONG
DIM N AS LONG

SUB Show (T AS STRING, Q AS LONG)
  DIM J AS INTEGER
  PRINT "len"; LEN(T); " pos"; Q; " [";
  FOR J = 1 TO LEN(T)
    PRINT ASC(MID$(T, J, 1)); " ";
  NEXT
  PRINT "]"
END SUB

M.WriteStr("ABCDEFGHIJ", 10)
PRINT "size"; M.Size; " pos"; M.Position
' the rest from 3, and more than is left
M.Position = 3
S = M.ReadStr(M.Size - M.Position)
Show(S, M.Position)
M.Position = 3
S = M.ReadStr(20)
Show(S, M.Position)
' at the end; nothing asked; a negative count
S = M.ReadStr(5)
Show(S, M.Position)
M.Position = 4
S = M.ReadStr(0)
Show(S, M.Position)
S = M.ReadStr(-2)
Show(S, M.Position)
' Position past the end and before the start
M.Position = 20
PRINT "set 20: pos"; M.Position; " size"; M.Size
S = M.ReadStr(3)
Show(S, M.Position)
M.Position = -3
PRINT "set -3: pos"; M.Position
S = M.ReadStr(4)
Show(S, M.Position)
M.WriteStr("XY", 2)
PRINT "write at -3: size"; M.Size; " pos"; M.Position
M.Seek(-7, soFromBeginning)
P = M.Position
PRINT "seek -7: pos"; P
M.Seek(5, soFromEnd)
PRINT "seek end+5: pos"; M.Position
' ReadLine near, at and past the end
M.Position = 8
S = M.ReadLine
Show(S, M.Position)
M.Position = 10
S = M.ReadLine
Show(S, M.Position)
M.Position = 14
S = M.ReadLine
Show(S, M.Position)
' numbers and Read(var) at the end
M.Position = 8
N = M.ReadNum(Num_LONG)
PRINT "readnum"; N; " pos"; M.Position
M.Position = 8
S = "1234"
M.Read(S)
Show(S, M.Position)
' writing past the end
M.Position = 12
M.WriteStr("Z", 1)
PRINT "write at 12: size"; M.Size; " pos"; M.Position
M.Position = 9
S = M.ReadStr(M.Size - M.Position)
Show(S, M.Position)
' Size and Position
M.Position = 9
M.Size = 4
P = M.Position
PRINT "pos 9, size 4: pos"; P; " size"; M.Size
M.Size = 6
P = M.Position
PRINT "size 6: pos"; P; " size"; M.Size
M.Position = 2
M.Size = 3
P = M.Position
PRINT "pos 2, size 3: pos"; P; " size"; M.Size
M.Position = 1
M.Size = 20
P = M.Position
PRINT "pos 1, size 20: pos"; P; " size"; M.Size

' a file stream: Position past the end, not before the start
DIM F AS QFILESTREAM
F.Open("tests/conformance/.work/memstream_out_of_range.txt", fmCreate)
F.WriteStr("abcdef", 6)
F.Position = 4
S = F.ReadLine
Show(S, F.Position)
F.Position = 9
PRINT "file set 9: pos"; F.Position; " size"; F.Size
F.WriteStr("Z", 1)
PRINT "file write at 9: size"; F.Size; " pos"; F.Position
F.Position = 4
N = F.ReadNum(Num_LONG)
PRINT "file readnum"; N; " pos"; F.Position
F.Position = -2
PRINT "file set -2: pos"; F.Position
F.Seek(-5, soFromBeginning)
PRINT "file seek -5: pos"; F.Position
F.Seek(3, soFromEnd)
PRINT "file seek end+3: pos"; F.Position
F.Close
