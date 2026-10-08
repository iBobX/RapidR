' QFILESTREAM's WriteByte / ReadByte (the .expected is RC.EXE's output,
' docs/rapidq-ground-truth.md): a value's low 8 bits; past the end ReadByte
' gives 26 (^Z, DOS's end-of-file mark) and Position stays.
$INCLUDE "RAPIDQ.INC"
DIM f AS QFILESTREAM
f.Open("tests/conformance/.work/filestream_bytes.bin", fmCreate)
f.WriteByte(65)
f.WriteByte(300)
f.WriteByte(-1)
PRINT "a "; f.Size; " "; f.Position
f.Position = 0
PRINT "b "; f.ReadByte; " "; f.ReadByte; " "; f.ReadByte; " "; f.Position
PRINT "c "; f.ReadByte; " "; f.Position
f.Position = 1
DIM b AS BYTE
b = f.ReadByte
PRINT "d "; b; " "; f.Position
f.Close
