' What a component's Color reads before the program sets it, as RapidQ has
' it — the .expected is RC.EXE's output (docs/rapidq-ground-truth.md):
' Windows' system colours, as Delphi TColors (&H80000000 + COLOR_…): a QFORM
' and a QPANEL clBtnFace (&H8000000F); a QLABEL, QCANVAS and QGROUPBOX their
' parent's Color, followed live (ParentColor), clWindow (&H80000005) with no
' parent; the others clWindow. What the program sets reads back as set. On
' screen clBtnFace is the theme's face (F0F0F0 on Windows 11 and in RapidR's
' classic theme).
$INCLUDE "RAPIDQ.INC"
DIM F AS QFORM
DIM P AS QPANEL
DIM L AS QLABEL
DIM C AS QCANVAS
DIM G AS QGROUPBOX
DIM B AS QBUTTON
DIM E AS QEDIT
PRINT "a "; HEX$(F.Color); " "; HEX$(P.Color); " "; HEX$(L.Color); " "; HEX$(C.Color); " "; HEX$(G.Color); " "; HEX$(B.Color); " "; HEX$(E.Color)
PRINT "b "; F.Color; " "; L.Color; " "; (F.Color = clBtnFace); " "; (L.Color = clWindow)
L.Parent = F: P.Parent = F: C.Parent = F: G.Parent = F: E.Parent = F
PRINT "c "; HEX$(L.Color); " "; HEX$(P.Color); " "; HEX$(C.Color); " "; HEX$(G.Color); " "; HEX$(E.Color)
F.Color = &HFF
PRINT "d "; HEX$(F.Color); " "; HEX$(L.Color); " "; HEX$(P.Color); " "; HEX$(C.Color); " "; HEX$(G.Color)
L.Color = &HFF00
F.Color = &HFF0000
PRINT "e "; HEX$(L.Color); " "; HEX$(C.Color)
DIM C2 AS QCANVAS
C2.Parent = G
G.Color = &H123456
PRINT "f "; HEX$(C2.Color)
P.Color = clWindow
PRINT "g "; HEX$(P.Color)
P.Color = 5
PRINT "h "; HEX$(P.Color)
CREATE Win AS QFORM
  CREATE Lbl AS QLABEL
    Caption = "x"
  END CREATE
END CREATE
PRINT "i "; HEX$(Win.Color); " "; HEX$(Lbl.Color)
DIM LB AS QLISTBOX
DIM CO AS QCOMBOBOX
DIM SG AS QSTRINGGRID
PRINT "j "; HEX$(LB.Color); " "; HEX$(CO.Color); " "; HEX$(SG.Color)
