' The pure maths builtins the interpreter calls itself (docs/vm-fast-design.md:
' ABS, INT, FIX, FRAC, SGN, SQR, SIN, COS, TAN, ATN, LOG, EXP, CEIL, FLOOR,
' ROUND, CINT, CLNG, CDBL, CSNG) at the edges: halves, negatives, -0,
' beyond 32 bits, text, a comma-separated list to SIN, and in typed
' expressions. The answers are the interpreter's before (2.122) and native
' builds'.
DIM d AS DOUBLE, i AS INTEGER, t AS DOUBLE
PRINT "int "; INT(2.5); " "; INT(-2.5); " "; INT(-0.5); " "; INT(1E10); " "; INT(7)
PRINT "fix "; FIX(-2.7); " "; FIX(2.7); " frac "; FRAC(-2.75); " "; FRAC(3)
PRINT "abs "; ABS(-3); " "; ABS(-2.5); " "; ABS(0); " "; ABS(-2147483648)
PRINT "sgn "; SGN(-0.1); " "; SGN(0); " "; SGN(5)
PRINT "sqr "; SQR(2); " "; SQR(16); " "; SQR(0)
PRINT "round "; ROUND(2.5); " "; ROUND(-2.5); " "; ROUND(-2.2); " "; ROUND(1E10)
PRINT "cint "; CINT(0.5); " "; CINT(-3.99); " clng "; CLNG(2147483647.6)
PRINT "ceil "; CEIL(2.1); " "; CEIL(-2.1); " floor "; FLOOR(2.9); " "; FLOOR(-2.1)
PRINT "trig "; SIN(0); " "; COS(0); " "; INT(SIN(1) * 1000000); " "; INT(COS(2) * 1000000); " "; INT(TAN(0.5) * 1000000); " "; INT(ATN(1) * 1000000)
PRINT "log "; INT(LOG(10) * 1000000); " "; INT(EXP(1) * 1000000)
PRINT "conv "; CDBL(3); " "; CSNG(0.1)
PRINT "text "; ABS("-4"); " "; INT("2.5"); " "; SQR("9")
PRINT "list "; SIN("0,0")
d = -1.5: i = 3
t = ABS(d) * i + INT(d) - SGN(d): PRINT "expr "; t
t = 0
FOR i = 1 TO 100: t = t + ABS(SIN(i)) + SQR(i): NEXT
PRINT "loop "; INT(t * 1000)
