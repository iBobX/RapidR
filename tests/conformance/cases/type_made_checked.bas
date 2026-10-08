' A TYPE a DIM makes is compiled (RC.EXE): its method's call of an
' undeclared routine is an error, and so is a QRECT in a plain TYPE.
TYPE TA EXTENDS QOBJECT
  A AS INTEGER
  SUB Foo
    Bar(TA.A, 2)
  END SUB
END TYPE
TYPE TB
  R AS QRECT
END TYPE
DIM v AS TA
DIM w AS TB
