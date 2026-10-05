' RapidQ's compiler (RC.EXE) on its data types QRECT and QNOTIFYICONDATA:
' a member they don't have, one assigned to another, one in a TYPE without
' EXTENDS (a STRUCT), cbSize stored.
TYPE TT
  R AS QRECT
END TYPE
DIM r AS QRECT
DIM r2 AS QRECT
DIM n AS QNOTIFYICONDATA
PRINT r.Width
r.Tag = 1
r2 = r
n.cbSize = 5
PRINT n.Tag
