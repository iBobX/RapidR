' A type name nothing defines, in RapidQ's compiler's (RC.EXE's) words for
' each place one can stand (probes 2026-10-08): a DIM (in a SUB too, an
' array too), a SUB's or FUNCTION's parameter, a FUNCTION's result, a
' CREATE, an EXTENDS, a field of an object TYPE and of a plain one.
' RapidQ's QGLobject examples DIM a QBITMAPEX.
DIM b AS QBITMAPEX
DIM a(5) AS FOOBAR
DIM i AS INTEGER, j AS FOOBAR
SUB S(p AS FOOBAR)
  DIM q AS FOOBAR
END SUB
FUNCTION F AS FOOBAR
  F = 1
END FUNCTION
CREATE c AS FOOBAR
END CREATE
TYPE T1 EXTENDS FOOBAR
  x AS INTEGER
END TYPE
TYPE T2 EXTENDS QOBJECT
  x AS FOOBAR
END TYPE
TYPE T3
  x AS FOOBAR
END TYPE
DIM v1 AS T1
DIM v2 AS T2
DIM v3 AS T3
