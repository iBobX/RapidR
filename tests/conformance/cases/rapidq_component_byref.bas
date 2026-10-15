' A component passed to a BYREF parameter (`BYREF ptable AS QSTRINGGRID`,
' RapidQ's sqlite3.inc): the routine fills the caller's grid. Native builds
' failed ("cannot find value"). The .expected is RC.EXE's output.
$APPTYPE CONSOLE
DIM Table AS QSTRINGGRID
FUNCTION Fill (n AS INTEGER, BYREF G AS QSTRINGGRID) AS INTEGER
  G.Cell(1, 1) = "x" + STR$(n)
  Result = G.RowCount
END FUNCTION
r = Fill(3, Table)
PRINT r; " "; Table.Cell(1, 1)
