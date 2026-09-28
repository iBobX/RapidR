' STRING * n: a store is cut to n characters (variables, arrays, TYPE fields, locals)
DIM s AS STRING * 8
DIM a(2) AS STRING * 3
TYPE Rec
  Name AS STRING * 5
  Age AS INTEGER
END TYPE
DIM r AS Rec
DIM t AS STRING * 4

SUB Local
  DIM z AS STRING * 2
  z = "abcdef"
  PRINT "[" + z + "]"
END SUB

s = "hello world"
PRINT "[" + s + "]", LEN(s)
s = "hi"
PRINT "[" + s + "]", LEN(s)
a(0) = "abcdef"
a(1) = "xy"
PRINT a(0); a(1); "|"; a(2); "|"
r.Name = "Bobby Tables"
r.Age = 3
PRINT "[" + r.Name + "]", r.Age
t = "ab"
t = t + "cdefg"
PRINT t
Local
PRINT "[" + t + "]"
DIM u AS STRING * 0
u = "unbounded"
PRINT u
PRINT CBOOL(5), CBOOL(0), CBOOL("x"), CBOOL("0"), CBOOL("")
IF CBOOL(2.5) THEN PRINT "yes" ELSE PRINT "no"
