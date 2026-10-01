' RapidQ's compile-time type check: a string stored into a number is an
' error, worded as RapidQ's compiler words it (undeclared n is a DOUBLE).
DIM count AS INTEGER, s AS STRING
s = "12"
count = "12"
n = "x" + s
count = VAL(s)
PRINT count
