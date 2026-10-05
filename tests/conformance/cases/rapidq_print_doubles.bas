' Numbers as RapidQ's PRINT and STR$ show them — the .expected is RC.EXE's
' output (docs/rapidq-ground-truth.md). PRINT: a whole number as a 32-bit
' integer (beyond 32 bits, an infinity or NaN: -2147483648), any other with 9
' decimals (Delphi's FloatToDecimal digits); STR$: 9 significant digits, the
' shorter of fixed and scientific, no leading space. SINGLE is a 32-bit float.
PRINT "a "; 1/3; " "; 2/3; " "; 10/4; " "; 0.1; " "; 0.1 + 0.2; " "; -0.5; " "; 3.0
PRINT "b "; 1/7*1000000; " "; 100/3; " "; -100/3; " "; 2^0.5; " "; 2^10; " "; 7/2
PRINT "c "; 1E-20; " "; -1E-20; " "; 0.000001; " "; 0.0000000005; " "; 0.9999999999; " "; -0.9999999996
PRINT "d "; 123456789.1; " "; 1234567890.1; " "; 1E9 + 0.5; " "; 2147483647.5; " "; 99999999999.5
PRINT "e "; 1234567.123456789; " "; 9876543.987654321; " "; 9999999.99999999; " "; 123456.1234567895; " "; 1.0000000015
PRINT "f "; 2147483647; " "; 2147483648; " "; 1E10; " "; 1E20; " "; -3000000000; " "; 1.5E9
DIM i AS INTEGER
DIM s AS SINGLE
DIM d AS DOUBLE
i = 7
PRINT "g "; i / 2; " "; i \ 2; " "; i * 1.5; " "; i; i; " "; -i
s = 0.1
PRINT "h "; s; " "; s * 3; " "; s + 0.2; " "; s * 1
s = 1/3
PRINT "i "; s; " "; STR$(s)
s = 16777217
PRINT "j "; s
s = 123456.789
PRINT "k "; s
d = 0
PRINT "l "; 7 / d; " "; 0 / d; " "; -d
d = 1E300
PRINT "m "; d * d; " "; STR$(d * d)
PRINT "n ["; STR$(1/3); "] ["; STR$(5); "] ["; STR$(-5); "] ["; STR$(2.5); "] ["; STR$(1E20); "] ["; STR$(0.1); "]"
PRINT "o ["; STR$(123456.789); "] ["; STR$(1/7*1000000); "] ["; STR$(1234567890.5); "] ["; STR$(0.00001); "] ["; STR$(0.0001); "]"
PRINT "p ["; STR$(0.000001234); "] ["; STR$(3000000000); "] ["; STR$(2147483648); "] ["; STR$(123456789.123); "] ["; STR$(100/3); "]"
i = 1234567890
PRINT "q ["; STR$(i); "] "; i
PRINT "r "; 1, 2, "x", -2.5
d = 0
d = d / d
PRINT "u "; d = d; " "; d <> d; " "; d < 1; " "; d > 1; " "; d <= 1; " "; d >= 1
DIM k AS INTEGER
FOR k = 1 TO 3
  PRINT "s "; k / 10
NEXT
FOR d = 0 TO 0.35 STEP 0.1
  PRINT "t "; d
NEXT
