' RC.EXE can't store into a field of an element of an array of objects
' (a TYPE EXTENDS QOBJECT) nor take one in WITH (probes
' 2026-10-08; RapidQ's games/WIP_asteroids3D.bas stops at such a line).
' A plain TYPE's array and a component array are fine.
TYPE S EXTENDS QOBJECT
  x AS INTEGER
END TYPE
TYPE P
  x AS INTEGER
END TYPE
DIM arr(3) AS S
DIM pl(3) AS P
DIM L(2) AS QSTRINGLIST
DIM b AS INTEGER
arr(1).x = 5
pl(1).x = 5
L(1).Sorted = 1
PRINT arr(1).x
WITH arr(b)
  PRINT .x
END WITH
