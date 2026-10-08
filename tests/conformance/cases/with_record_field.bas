' WITH on a QRECT field of an object a TYPE makes (RapidQ's direct3d
' examples keep QRECTs in TYPEs): its .Member reads and stores reach the
' field, as v.R.Bottom does (checked against RC.EXE).
$APPTYPE CONSOLE
TYPE TT EXTENDS QOBJECT
  A AS INTEGER
  R AS QRECT
END TYPE
DIM v AS TT
WITH v.R
  .Bottom = 99
  .Left = 3
  PRINT .Bottom; " "; .Left
END WITH
PRINT v.R.Bottom; " "; v.R.Left
DIM q AS QRECT
WITH q
  .Right = 5
END WITH
PRINT q.Right
