' errors.bas: a program with an error, for the diagnostics test. With
' $TYPECHECK ON, RapidQ's compiler says "Undeclared identifier undeclaredThing".
$TYPECHECK ON

DIM total AS INTEGER
total = 1
undeclaredThing = 2
PRINT total
