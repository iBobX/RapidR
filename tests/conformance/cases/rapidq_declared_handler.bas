' An event bound to a SUB the program only DECLAREs fires nothing; the
' program builds and runs — the .expected is RC.EXE's output (RapidQ's
' QStringGridsTwoLinesBitMap example binds its OnSelectCell so).
DECLARE SUB Missing (Col%, Row%, CanSelect%)
DECLARE SUB Nothing
DIM G AS QSTRINGGRID
G.OnSelectCell = Missing
CREATE F AS QFORM
  OnShow = Nothing
END CREATE
PRINT "ok"
