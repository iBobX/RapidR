' More forms RapidQ's example programs use that RC.EXE takes (the
' .expected is RC.EXE's output): a fixed-length string whose length is a
' product of CONSTs (`STRING * MAX_PATH * 2`), a DEFSTR with nothing after
' its `=`, a `-` with nothing after it (`x > -` is -x, `5 + -` is -5), `#`
' before a name (`#H1F` is the undeclared variable H1F), a SUB named like a
' directive (`SUB $XP_MANIFEST`, called as `$XP_MANIFEST`), and END SUB
' ending a TYPE's PROPERTY SET.
$APPTYPE CONSOLE
CONST MAX_PATH = 3
DIM w AS STRING *MAX_PATH * 2
PRINT LEN(w)
w = "abcdefghij"
PRINT w
DefStr sMsg =
PRINT "["; sMsg; "]"; LEN(sMsg)
DIM b AS INTEGER
b = 3
IF b > - THEN PRINT "gt"
b = 0
IF b > - THEN PRINT "zero"
PRINT 5 + -
DIM c AS LONG
c = #H1F
PRINT c
SUB $HELLO
  PRINT "hello sub"
END SUB
$HELLO
TYPE TT EXTENDS QOBJECT
  V AS INTEGER PROPERTY SET SetV
  PROPERTY SET SetV (x AS INTEGER)
    This.V = x * 2
  END SUB
END TYPE
DIM o AS TT
o.V = 4
PRINT o.V
