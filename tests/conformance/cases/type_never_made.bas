' RapidQ's compiler compiles a TYPE where a DIM makes one (RC.EXE): a
' TYPE nothing makes is never compiled — its methods may call what isn't
' declared, a plain TYPE may hold a QRECT, a field may name a type that
' doesn't exist; a parameter of the type makes none. RapidQ's
' direct3d/Lights_pyramid.bas carries such a TYPE (QD3DCamera).
$APPTYPE CONSOLE
TYPE TA EXTENDS QOBJECT
  A AS INTEGER
  SUB Foo
    Bar(TA.A, 2)
  END SUB
END TYPE
TYPE TB
  A AS INTEGER
  R AS QRECT
END TYPE
TYPE TC EXTENDS QOBJECT
  A AS NOSUCHTYPE
END TYPE
SUB Z (p AS TA)
  PRINT "z"
END SUB
TYPE TD EXTENDS QOBJECT
  A AS INTEGER
  SUB Foo
    PRINT "foo "; TD.A
  END SUB
END TYPE
DIM v AS TD
DIM w AS TD
v.A = 1: w.A = 2
v.Foo: w.Foo
PRINT "ok"
