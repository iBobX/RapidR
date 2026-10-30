' QFILESTREAM's ReadStr(n) reads n characters and moves n on, but hands back
' n + 1: the last one a space. ReadBinStr(n) gives exactly n. A QMEMORYSTREAM's
' ReadStr gives n. The .expected is RC.EXE's output (docs/rapidq-ground-truth.md;
' filestream_reads has the edges: a count of 0, past the end, ReadLine).
$INCLUDE "RAPIDQ.INC"
$APPTYPE CONSOLE
DIM F AS QFILESTREAM
DIM M AS QMEMORYSTREAM
DIM S AS STRING
DIM J AS INTEGER

SUB Show (Tag AS STRING, T AS STRING, P AS LONG)
  DIM K AS INTEGER
  PRINT Tag; " len"; LEN(T); " pos"; P; " [";
  FOR K = 1 TO LEN(T)
    PRINT ASC(MID$(T, K, 1)); " ";
  NEXT
  PRINT "]"
END SUB

F.Open("rapidq_readstr.dat", fmCreate)
F.WriteStr("abcdefgh", 8)
F.Close
F.Open("rapidq_readstr.dat", fmOpenRead)
S = F.ReadStr(3)
Show("str", S, F.Position)
S = F.ReadBinStr(3)
Show("bin", S, F.Position)
S = F.ReadStr(1)
Show("str1", S, F.Position)
F.Position = 0
S = F.ReadStr(F.Size)
Show("all", S, F.Position)
F.Close
KILL "rapidq_readstr.dat"
M.WriteStr("abcdefgh", 8)
M.Position = 0
S = M.ReadStr(3)
Show("mem", S, M.Position)
S = M.ReadBinStr(3)
Show("membin", S, M.Position)
