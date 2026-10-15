' QRICHEDIT's SaveToStream / LoadFromStream (the bytes SaveToFile writes,
' at the stream's position; read back from the position to the end):
' rich text (RTF, ending with a NUL byte) while PlainText is False, the
' text with PlainText True — RC.EXE's (probe w_rgs, its window run in the
' VM: the same sizes, bytes and lines); QSTRINGGRID's DeleteColumn
' (DeleteCol's other name in RC.EXE's table), QFONTDIALOG's GetFont (a
' QFONT's attributes into the dialog).
DIM M AS QMEMORYSTREAM
DIM R AS QRICHEDIT
DIM R2 AS QRICHEDIT
R.Text = "one" + CHR$(13) + CHR$(10) + "two"
M.WriteStr("<<", 2)
R.SaveToStream(M)
PRINT "size "; M.Size; " pos "; M.Position
M.Position = 0
S$ = M.ReadStr(M.Size)
PRINT "["; REPLACESUBSTR$(LEFT$(S$, LEN(S$) - 1), CHR$(13), ""); "] last "; ASC(RIGHT$(S$, 1))
M.Position = 2
R2.LoadFromStream(M)
PRINT "pos "; M.Position; " lines "; R2.LineCount
PRINT "[" + R2.Line(0) + "][" + R2.Line(1) + "]"
R.PlainText = 1
M.Position = 0
R.SaveToStream(M)
PRINT "plain size "; M.Size; " pos "; M.Position
DIM G AS QSTRINGGRID
G.ColCount = 4
G.Cell(1, 1) = "a": G.Cell(2, 1) = "b": G.Cell(3, 1) = "c"
G.DeleteColumn(2)
PRINT "cols "; G.ColCount; " "; G.Cell(1, 1); G.Cell(2, 1)
DIM D AS QFONTDIALOG
DIM Fnt AS QFONT
Fnt.Name = "Courier New"
Fnt.Size = 14
Fnt.Bold = 1
D.GetFont(Fnt)
PRINT D.Name; " "; D.Size
