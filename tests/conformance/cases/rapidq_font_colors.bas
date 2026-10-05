' Font.Color before the program sets it, as RapidQ has it — the .expected
' is RC.EXE's output (docs/rapidq-ground-truth.md): clWindowText
' (&H80000008) for every component and a new QFONT; once parented, the
' parent's Font.Color, followed live (Delphi's ParentFont) until the program
' sets its own.
DIM F AS QFORM
DIM L AS QLABEL
DIM B AS QBUTTON
DIM E AS QEDIT
DIM P AS QPANEL
DIM C AS QCANVAS
DIM G AS QGROUPBOX
DIM LB AS QLISTBOX
DIM CB AS QCHECKBOX
DIM RB AS QRADIOBUTTON
DIM CO AS QCOMBOBOX
DIM M AS QRICHEDIT
DIM SG AS QSTRINGGRID
PRINT "a "; HEX$(F.Font.Color); " "; HEX$(L.Font.Color); " "; HEX$(B.Font.Color); " "; HEX$(E.Font.Color); " "; HEX$(P.Font.Color); " "; HEX$(C.Font.Color); " "; HEX$(G.Font.Color)
PRINT "b "; HEX$(LB.Font.Color); " "; HEX$(CB.Font.Color); " "; HEX$(RB.Font.Color); " "; HEX$(CO.Font.Color); " "; HEX$(M.Font.Color); " "; HEX$(SG.Font.Color)
F.Font.Color = &HFF
L.Parent = F: B.Parent = F: E.Parent = F: P.Parent = F: C.Parent = F: G.Parent = F
PRINT "c "; HEX$(F.Font.Color); " "; HEX$(L.Font.Color); " "; HEX$(B.Font.Color); " "; HEX$(E.Font.Color); " "; HEX$(P.Font.Color); " "; HEX$(C.Font.Color); " "; HEX$(G.Font.Color)
F.Font.Color = &HFF00
PRINT "d "; HEX$(L.Font.Color); " "; HEX$(P.Font.Color)
L.Font.Color = &H123456
F.Font.Color = &HFF0000
PRINT "e "; HEX$(L.Font.Color); " "; HEX$(P.Font.Color)
DIM FN AS QFONT
PRINT "f "; HEX$(FN.Color)
