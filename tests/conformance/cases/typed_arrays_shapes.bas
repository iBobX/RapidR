' Typed arrays: native builds keep an array declared with a numeric type as
' a plain vector of its numbers (rapidr-codegen-rust typed_arrays), the
' interpreter as Values. Every type and its conversion on store, elements
' in expressions, RapidQ's unchecked subscripts (a place past one dimension
' is the next row's; past the whole array a store reads back, a place never
' written reads 0), LBOUND / UBOUND, REDIM. Both print exactly the same.
$APPTYPE CONSOLE
DIM d(1 TO 4) AS DOUBLE
DIM s(3) AS SINGLE
DIM n(3) AS INTEGER
DIM l(3) AS LONG
DIM b(3) AS BYTE
DIM w(3) AS WORD
DIM sh(3) AS SHORT
DIM dw(3) AS DWORD
DIM big(2) AS INT64
DIM ubig(2) AS UINT64
DIM k AS INTEGER, t AS DOUBLE, m AS INTEGER

d(1) = 1.5: d(2) = d(1) * 3: d(3) = 7 / 2: d(4) = "2.25"
FOR k = 1 TO 4
  PRINT d(k); " ";
NEXT
PRINT
s(0) = 0.1: s(1) = 1 / 3: s(2) = s(0) + s(1)
PRINT s(0); " "; s(1); " "; s(2)
n(0) = 2.7: n(1) = -2.7: n(2) = 1E10: n(3) = "42"
PRINT n(0); " "; n(1); " "; n(2); " "; n(3)
l(0) = 2147483647: l(1) = l(0) + 1
PRINT l(0); " "; l(1)
b(0) = 255: b(1) = b(0) + 1: b(2) = -1: b(3) = 3.9
PRINT b(0); " "; b(1); " "; b(2); " "; b(3)
w(0) = 65535: w(1) = w(0) + 1: w(2) = -1
PRINT w(0); " "; w(1); " "; w(2)
sh(0) = 32767: sh(1) = sh(0) + 1: sh(2) = -40000
PRINT sh(0); " "; sh(1); " "; sh(2)
dw(0) = -1: dw(1) = 3000000000
PRINT dw(0); " "; dw(1)
big(0) = 9007199254740993: big(1) = big(0) * 2: big(2) = big(1) - big(0)
PRINT big(0); " "; big(1); " "; big(2)
ubig(0) = 18446744073709551615: ubig(1) = ubig(0) - 1
PRINT ubig(0); " "; ubig(1); " "; ubig(1) < ubig(0)

' Elements in expressions: arithmetic, comparisons, logic, text.
t = d(1) + n(0) * b(0) - s(0)
PRINT t; " "; n(0) + n(1); " "; n(0) \ 2; " "; n(0) MOD 2; " "; d(3) ^ 2
PRINT d(2) > d(1); " "; n(0) = 2; " "; b(0) AND 15; " "; b(0) OR 256; " "; NOT n(0)
PRINT b(0) SHR 4; " "; b(3) SHL 30; " "; -1 SHR 28; " "; n(1) SHL 33
PRINT "d(1) is " + STR$(d(1)); " "; "n: " & n(0); " "; LEN(STR$(b(0)))
IF d(1) THEN PRINT "true element"
SELECT CASE n(0)
  CASE 1: PRINT "one"
  CASE 2: PRINT "two"
END SELECT
m = 1
WHILE n(m) < 0
  PRINT "negative at"; m
  m = m + 1
WEND

' SWAP, INC, DEC on elements
SWAP d(1), d(2)
INC n(0)
DEC n(3), 2
PRINT d(1); " "; d(2); " "; n(0); " "; n(3)

' Two and three dimensions; unchecked subscripts
DIM g(1 TO 3, 0 TO 4) AS INTEGER
DIM c(1, 2, 3) AS DOUBLE
FOR k = 1 TO 3
  FOR m = 0 TO 4
    g(k, m) = k * 10 + m
  NEXT
NEXT
PRINT g(2, 3); " "; g(1, 5); " "; g(3, -1); " "; g(4, 0)
g(3, 7) = 99
PRINT g(3, 7); " "; g(9, 9)
c(1, 2, 3) = 1.25: c(0, 0, 0) = -1
PRINT c(1, 2, 3); " "; c(0, 0, 0); " "; c(0, 0, 4)

' Past the end of the whole array: kept, read back
DIM p(2) AS DOUBLE
p(5) = 7.5: p(-1) = 2
PRINT p(5); " "; p(-1); " "; p(4); " "; p(3)

' LBOUND / UBOUND
PRINT LBOUND(d); " "; UBOUND(d); " "; LBOUND(g, 2); " "; UBOUND(g, 2); " "; UBOUND(c, 3)
t = 0
FOR k = LBOUND(d) TO UBOUND(d)
  t = t + d(k)
NEXT
PRINT "sum"; t

' REDIM keeps what is still inside; a place now outside reads back
DIM r(0 TO 4) AS INTEGER
FOR k = 0 TO 4
  r(k) = k + 100
NEXT
REDIM r(0 TO 7) AS INTEGER
PRINT UBOUND(r); " "; r(4); " "; r(7)
REDIM PRESERVE r(0 TO 2) AS INTEGER
PRINT UBOUND(r); " "; r(2); " "; r(4)
REDIM r(1 TO 2, 1 TO 2) AS INTEGER
PRINT UBOUND(r, 2); " "; r(1, 1); " "; r(2, 2)
' A DIM run again (in a loop): a new array of zeros each time
FOR k = 1 TO 2
  DIM q(0 TO 1) AS DOUBLE
  q(1) = q(1) + k
  PRINT "q"; q(1);
NEXT
PRINT
