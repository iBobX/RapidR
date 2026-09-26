' RapidQ's non-visual objects (Appendix A), shared by every runtime in
' rapidr_value::objects: QFONT assigned to a component, QMEMORYSTREAM
' reads/writes, QBITMAP drawing and BMP through a stream, QIMAGELIST strips.
$INCLUDE "RAPIDQ.INC"
DIM Font AS QFONT
DIM Label AS QLABEL
Font.Name = "Courier New"
Font.Size = 14
Font.AddStyles(fsBold, fsUnderline)
Font.DelStyles(fsUnderline)
Label.Font = Font
PRINT Label.FontName; ","; Label.FontSize; ","; Label.FontBold; ","; Font.Underline

DIM Mem AS QMEMORYSTREAM
S$ = "Hello world!"
Mem.WriteStr(S$, LEN(S$))
Mem.WriteNum(-2, Num_SHORT)
Mem.WriteNum(3.25, Num_DOUBLE)
Mem.WriteLine("line two")
PRINT Mem.Size; ","; Mem.Position; ","; Mem.LineCount
Mem.Position = 0
PRINT Mem.ReadStr(5); "|"; Mem.ReadStr(7); "|"; Mem.ReadNum(Num_SHORT); "|"; Mem.ReadNum(Num_DOUBLE)
PRINT Mem.ReadLine; "|"
Mem.Seek(-4, soFromEnd)
PRINT Mem.ReadStr(2)
DIM Copy AS QMEMORYSTREAM
Copy.CopyFrom(Mem, 0)
PRINT Copy.Size
Mem.Close
PRINT Mem.Size

DIM Bmp AS QBITMAP
Bmp.Width = 20
Bmp.Height = 10
Bmp.FillRect(0, 0, 20, 10, clRed)
Bmp.Circle(2, 2, 9, 9, clBlue, 1)
Bmp.Line(0, 9, 19, 9, clBlack)
Bmp.Pset(19, 0, clLime)
Bmp.Pixel(0, 0) = clYellow
PRINT HEX$(Bmp.Pixel(5, 5)); " "; HEX$(Bmp.Pixel(19, 0)); " "; HEX$(Bmp.Pixel(0, 0)); " "; HEX$(Bmp.Pixel(10, 9)); " "; Bmp.Pixel(50, 50)

DIM Stream AS QMEMORYSTREAM
Bmp.SaveToStream(Stream)
PRINT Stream.Size
Stream.Position = 0
DIM Loaded AS QBITMAP
Loaded.LoadFromStream(Stream)
PRINT Loaded.Width; "x"; Loaded.Height; " "; HEX$(Loaded.Pixel(19, 0))

DIM Icons AS QIMAGELIST
Icons.Width = 10
Icons.Height = 10
Icons.AddBMPHandle(Bmp.BMP, clRed)
PRINT Icons.Count
DIM Second AS QBITMAP
Second.BMP = Icons.GetBMP(1)
PRINT Second.Width; " "; HEX$(Second.Pixel(9, 0))
Icons.Delete(0)
PRINT Icons.Count
