' Operators and precedence.
PRINT "add:" + STR$(2 + 3 * 4)
PRINT "paren:" + STR$((2 + 3) * 4)
PRINT "intdiv:" + STR$(17 \ 5)
PRINT "mod:" + STR$(17 MOD 5)
PRINT "pow:" + STR$(2 ^ 10)
PRINT "div:" + STR$(7 / 2)
PRINT "neg:" + STR$(-3 + 1)
PRINT "concat:" + "a" & "b"
IF 3 > 2 AND NOT (1 > 2) THEN PRINT "logic:ok"
IF "abc" < "abd" THEN PRINT "strcmp:ok"
