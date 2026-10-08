' A QRECT field of an object is a QRECT (RC.EXE): another QRECT can't be
' assigned to it, and it has only Left, Top, Right and Bottom.
TYPE TT EXTENDS QOBJECT
  R AS QRECT
END TYPE
DIM v AS TT
DIM r2 AS QRECT
v.R = r2
PRINT v.R.Width
