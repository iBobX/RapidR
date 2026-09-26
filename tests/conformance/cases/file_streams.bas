' QFILESTREAM on the shared stream code (the same methods as QMEMORYSTREAM):
' fmCreate/fmOpenRead/fmOpenReadWrite, WriteLine/ReadLine, numbers, Read(var),
' EOF as True (-1) so WHILE NOT File.EOF ends, LineCount, Seek, CopyFrom.
$INCLUDE "RAPIDQ.INC"
F$ = "tests/conformance/.work/file_streams.txt"
DIM File AS QFILESTREAM
File.Open(F$, fmCreate)
File.WriteLine("first line")
File.WriteLine("second line")
File.WriteNum(12345, Num_LONG)
File.WriteStr("tail", 4)
File.Close

File.Open(F$, fmOpenRead)
PRINT "size"; File.Size; " lines"; File.LineCount
PRINT File.ReadLine
DIM Second AS STRING
Second = SPACE$(6)
File.Read(Second)
PRINT "[" + Second + "]"
PRINT File.ReadLine
DIM N AS LONG
File.Read(N)
PRINT N
n = 0
WHILE NOT File.EOF
  PRINT File.ReadStr(1);
  n = n + 1
WEND
PRINT " /"; n; File.EOF
File.WriteLine("refused")
File.Close

File.Open(F$, fmOpenReadWrite)
File.Seek(0, soFromEnd)
File.WriteLine("!")
DIM Mem AS QMEMORYSTREAM
Mem.CopyFrom(File, 0)
PRINT "copied"; Mem.Size
Mem.Position = Mem.Size - 3
PRINT Mem.ReadLine
File.Close
