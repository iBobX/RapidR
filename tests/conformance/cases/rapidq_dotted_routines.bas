' xfail: codegen — native builds call a dotted SUB / FUNCTION with C-SYS-2's dotted_routine (crates/rapidr-codegen-rust): remove this line when that lane is merged
' SUBs and FUNCTIONs named with a dot, as RapidQ lets a program name them
' (RapidQ's 3DBOX example: SUB Draw.3DBox, called Draw.3DBox 1, 1, ...):
' a call is the routine's, not a method of an object; a FUNCTION returns
' what's assigned to its whole name. The .expected is RC.EXE's output.
$APPTYPE CONSOLE
SUB Draw.Box (A AS INTEGER)
  PRINT "box"; A
END SUB
SUB Draw.3DBox (A AS INTEGER, B AS INTEGER)
  PRINT "3d"; A; B
END SUB
FUNCTION Calc.Twice (N AS INTEGER) AS INTEGER
  Calc.Twice = N * 2
END FUNCTION
FUNCTION Calc.Thrice (N AS INTEGER) AS INTEGER
  Result = N * 3
END FUNCTION
Draw.Box 1
Draw.3DBox 2, 3
Draw.Box(4)
PRINT Calc.Twice(21); " "; Calc.Twice(4) + 1; " "; Calc.Thrice(2)
CALL Draw.Box(5)
