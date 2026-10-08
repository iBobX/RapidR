' ReadAll (a RapidR extension): the rest of a stream from Position, Position
' at the end after it — what ReadStr(Size - Position) reads, without the
' spaces RapidQ's ReadStr adds where there are no bytes: nothing at or past
' the end, nor before the start (memstream_out_of_range.bas has RapidQ's
' reads there). Past the start it once overflowed: empty in release builds,
' a panic in debug native builds.
$INCLUDE "RAPIDQ.INC"
DIM M AS QMEMORYSTREAM
DIM S AS STRING
M.WriteStr("ABCDEFGHIJ", 10)
M.Position = 0
PRINT "["; M.ReadAll; "]"; M.Position
M.Position = 3
PRINT "["; M.ReadAll; "]"; M.Position
M.Position = 4
S = M.ReadStr(2)
S = M.ReadAll
PRINT "["; S; "]"; M.Position
M.Position = 9
PRINT "["; M.ReadAll; "]"; M.Position
PRINT "["; M.ReadAll; "]"; M.Position
M.Position = 25
PRINT "["; M.ReadAll; "]"; M.Position
M.Position = -3
PRINT "["; M.ReadAll; "]"; M.Position
M.Seek(-4, soFromEnd)
PRINT "["; M.ReadAll(); "]"; M.Position

DIM F AS QFILESTREAM
F.Open("tests/conformance/.work/memstream_readall.txt", fmCreate)
F.WriteLine("first")
F.WriteLine("second")
F.Position = 0
PRINT "["; F.ReadLine; "]"
S = F.ReadAll
PRINT "["; LEFT$(S, 6); "] len"; LEN(S); " pos"; F.Position; " eof"; F.EOF
F.Close
