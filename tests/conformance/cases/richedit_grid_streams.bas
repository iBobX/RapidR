' QRICHEDIT's SaveToStream / LoadFromStream (the bytes SaveToFile writes,
' at the stream's position; read back from the position to the end),
' QSTRINGGRID's DeleteColumn (DeleteCol's other name in RC.EXE's table),
' QFONTDIALOG's GetFont (a QFONT's attributes into the dialog). Visual
' components: RC.EXE only compiles this here (its programs that make
' controls aren't run in the VM), so the expected output is RapidR's,
' following RapidQ's manual.
DIM M AS QMEMORYSTREAM
DIM R AS QRICHEDIT
DIM R2 AS QRICHEDIT
R.Text = "one" + CHR$(13) + CHR$(10) + "two"
M.WriteStr("<<", 2)
R.SaveToStream(M)
PRINT "size "; M.Size; " pos "; M.Position
M.Position = 0
PRINT "["; M.ReadStr(M.Size); "]"
M.Position = 2
R2.LoadFromStream(M)
PRINT "pos "; M.Position; " lines "; R2.LineCount
PRINT "[" + R2.Line(0) + "][" + R2.Line(1) + "]"
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
