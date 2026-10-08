DIM Form AS QFORM
DIM L AS QLISTBOX
DIM n AS RNUM
DIM p AS QPLOT
Form.ShapeForm "a.bmp", 0
L.Circle 1, 2, 3, 4, 0, 0
Form.Anchors = 0
BEEP
x = Screen.W|idth
y = Screen.|
Form.Sha|peForm "b.bmp", 0
LOCATE 1, |2
DIM T AS QTREEVIEW
z = T.Item(0).|
OPEN "f.txt" FOR INPUT AS #1
DIM r AS RBUTTON
'! 5 completion has Text Expanded ImageIndex
'! diagnostic main.bas:15:1 "OPEN is a RapidR extension: RapidQ's compiler refuses it (this project is RapidQ-compatible)"
'! options rapidq-compatible
'! diagnostic main.bas:5:6 "RapidQ's QForm.ShapeForm is not implemented in RapidR yet"
'! diagnostic main.bas:7:6 "QForm.Anchors is a RapidR extension: RapidQ's compiler refuses it (this project is RapidQ-compatible)"
'! diagnostic main.bas:8:1 "BEEP is a RapidR extension: RapidQ's compiler refuses it (this project is RapidQ-compatible)"
'! diagnostic main.bas:3:10 "RNum is RapidR's own component: RapidQ doesn't have it (this project is RapidQ-compatible)"
'! diagnostic main.bas:4:10 "RapidQ has no QPLOT: RapidR reads it as RPlot"
'! diagnostic main.bas:16:10 "RButton is RapidR's name: RapidQ's compiler knows it as QBUTTON"
'! 1 hover has "Screen.Width" "read only"
'! 2 completion has Width Height MouseX
'! 3 hover has "not implemented"
'! 3 completion lacks ShapeForm
'! 4 signature "LOCATE [Y%][, X%][, cursor]" active 1
