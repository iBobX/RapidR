' $TYPECHECK ON: a variable must be declared before it is stored into
' (RapidQ's "Undeclared identifier"); $TYPECHECK OFF relaxes it again, and
' $OPTION EXPLICIT is the same as $TYPECHECK ON.
$TYPECHECK ON
DIM a AS LONG
a = 1
b = 2
$TYPECHECK OFF
c = 3
$OPTION EXPLICIT
SUB S (p AS LONG)
  p = 1
  d = 4
END SUB
