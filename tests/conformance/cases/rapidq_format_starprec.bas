' `*` takes a width from the arguments, but not a precision: `%.*f` stops the
' program with EConvertError "Format '%.*f' invalid or incompatible with
' argument." The .expected is RC.EXE's output (docs/rapidq-ground-truth.md,
' "FORMAT$").
$APPTYPE CONSOLE
PRINT FORMAT$("[%*d]", 5, 42)
PRINT "before"
PRINT FORMAT$("%.*f", 1, 3.14159)
PRINT "after"
