' $TYPECHECK ON: an undeclared name read anywhere (a method's argument, an
' operand, a condition, what PRINT prints, a SUB's or FUNCTION's argument)
' is RC.EXE's "Undefined symbol ITME" (the name in upper case, its suffix
' kept); a store into one is "Undeclared identifier" (typecheck_error).
' RC.EXE (RapidQ 2006, in the Windows VM: docs/rapidq-ground-truth.md)
' stopped with that message at each of these lines, one probe per line;
' with $TYPECHECK OFF `L.AddItems itme` compiles (ItemCount 1).
$TYPECHECK ON
DIM L AS QSTRINGLIST
DIM x AS INTEGER, s AS STRING
SUB Show2 (a AS INTEGER)
  PRINT a
END SUB
FUNCTION Twice (a AS INTEGER) AS INTEGER
  Twice = a * 2
END FUNCTION
SUB Fill
  L.AddItems zz
END SUB
L.AddItems itme
L.AddItems "a", other
PRINT shown
x = gone + 1
Show2 arg
x = Twice(farg)
IF cond THEN PRINT "y"
L.AddItems name$
s = L.Item(index)
