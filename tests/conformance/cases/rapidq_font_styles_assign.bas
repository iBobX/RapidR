' `Font.AddStyles = fsBold` (an assignment) is the method's call in RapidQ —
' the .expected is RC.EXE's output (RapidQ's Sokoban examples write it so).
DIM L AS QLABEL
L.Font.AddStyles = 0
PRINT L.Font.Bold
DIM F AS QFONT
F.AddStyles = 1
PRINT F.Italic
F.DelStyles = 1
PRINT F.Italic
DIM B AS QBUTTON
B.Font.AddStyles = 2
PRINT B.Font.Underline
