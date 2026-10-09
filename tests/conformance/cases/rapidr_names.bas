' RapidR's names are the same components as RapidQ's: rapidq_objects.bas
' written with RapidR's names (RFont, RLabel, RMemoryStream, RBitmap,
' RImageList), as `rapidr upgrade-names` writes it, prints the same; and the
' two kinds of names mix in one program, in any case.
$INCLUDE "RAPIDQ.INC"
DIM Font AS RFont
DIM Label AS RLabel
Font.Name = "Courier New"
Font.Size = 14
Font.AddStyles(fsBold, fsUnderline)
Font.DelStyles(fsUnderline)
Label.Font = Font
PRINT Label.FontName; ","; Label.FontSize; ","; Label.FontBold; ","; Font.Underline

DIM Mem AS RMemoryStream
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
DIM Copy AS RMemoryStream
Copy.CopyFrom(Mem, 0)
PRINT Copy.Size
Mem.Close
PRINT Mem.Size

DIM Bmp AS RBitmap
Bmp.Width = 20
Bmp.Height = 10
Bmp.FillRect(0, 0, 20, 10, clRed)
Bmp.Circle(2, 2, 9, 9, clBlue, clBlue)
Bmp.Line(0, 9, 19, 9, clBlack)
Bmp.Pset(19, 0, clLime)
Bmp.Pixel(0, 0) = clYellow
PRINT HEX$(Bmp.Pixel(5, 5)); " "; HEX$(Bmp.Pixel(19, 0)); " "; HEX$(Bmp.Pixel(0, 0)); " "; HEX$(Bmp.Pixel(10, 9)); " "; Bmp.Pixel(50, 50)

DIM Stream AS RMemoryStream
Bmp.SaveToStream(Stream)
PRINT Stream.Size
Stream.Position = 0
DIM Loaded AS RBitmap
Loaded.LoadFromStream(Stream)
PRINT Loaded.Width; "x"; Loaded.Height; " "; HEX$(Loaded.Pixel(19, 0))

DIM Icons AS RImageList
Icons.Width = 10
Icons.Height = 10
Icons.AddBMPHandle(Bmp.BMP, clRed)
PRINT Icons.Count
DIM Second AS RBitmap
Second.BMP = Icons.GetBMP(1)
PRINT Second.Width; " "; HEX$(Second.Pixel(9, 0))
Icons.Delete(0)
PRINT Icons.Count
DIM A AS QLABEL, B AS rlabel
A.Caption = "q": B.Caption = "r"
SUB Show (L AS RLabel)
    PRINT L.Caption;
END SUB
Show A: Show B: PRINT
TYPE TCounter EXTENDS RObject
    N AS INTEGER
    SUB Add
        This.N = This.N + 1
    END SUB
END TYPE
DIM C AS TCounter
C.Add: C.Add
PRINT C.N
