' Hex in DATA and in strings: a DATA item &H10 is the text "&H10", read into a
' number it is 0 (READ x gives 0, a string or a VARIANT keeps the text), and
' VAL does not read &H, &O, &B, 0x or $ (0) — only the &H written in the
' program's own code is a number. The .expected is RC.EXE's output
' (docs/rapidq-ground-truth.md). (RapidR's own &O17 and &B101 literals are
' RapidR's addition: RapidQ reads them as the name O17 / B101, so they aren't here.)
$APPTYPE CONSOLE
DIM x AS INTEGER
DIM d AS DOUBLE
DIM s AS STRING
DIM t AS STRING
DIM v AS VARIANT
DATA &H10, &HFF, &H10, &H10
DATA &O17, 5, 1E2, -3
DATA &H10
DATA "12"
DATA -&H10
READ x
PRINT "int "; x
READ d
PRINT "dbl "; d
READ s
PRINT "str "; s
READ v
PRINT "var "; v
READ x
PRINT x
READ x
PRINT x
READ x
PRINT x
READ x
PRINT x
READ s
PRINT VAL(s)
READ x
PRINT x
READ x
PRINT x
PRINT VAL("&HFF"); VAL("&hff"); VAL("&H10"); VAL("0xFF"); VAL("&O17"); VAL("&B101"); VAL("$FF")
PRINT VAL("12"); VAL("1e3"); VAL(" 7")
s = "&HFF"
x = VAL(s)
PRINT x
t = "&H10"
PRINT VAL(t) + 1
PRINT s + t
PRINT &H10; &HFF; &H10 + 1
