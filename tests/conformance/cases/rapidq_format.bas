' FORMAT$ as RapidQ's Format does it — the .expected is RC.EXE's output
' (docs/rapidq-ground-truth.md, "FORMAT$"): the arguments' types count (%s
' shows a string and nothing for a number; %d %x %f … read a string as 0),
' %d and %x take a 32-bit integer (cut toward zero, -2147483648 beyond), the
' digits are RapidQ's own 18 (2.675 rounds to 2.67), a negative number
' rounded to zero keeps its minus while the rounding reaches its digits, a
' precision above 15 (%d %x) or 18 (%e %f %g %n %m) is ignored, %.0e and %.0g
' show 2 digits, a % ending the text is dropped. A refused specifier stops the
' program (rapidq_format_plus, rapidq_format_ld, rapidq_format_starprec).
$APPTYPE CONSOLE
DIM d AS DOUBLE
DIM z AS DOUBLE
DIM s AS STRING, s2 AS STRING, s3 AS STRING
DIM v AS VARIANT
DIM n AS INTEGER

SUB Show (Tag AS STRING, T AS STRING)
  PRINT Tag; " ["; T; "]"
END SUB

s = "text"
v = "12"
n = 42
z = 0

' integers
Show("d", FORMAT$("%d|%5d|%-5d|%05d|%.5d|%10.5d|%-10.5d|", n, n, n, n, n, n, n))
Show("neg", FORMAT$("%d|%5d|%05d|%.5d", -42, -42, -42, -42))
Show("x", FORMAT$("%x|%X|%02X|%.2X|%04x|%.4x|%8X|%-8X|%08X|%2X", 255, 255, 10, 10, 10, 10, 255, 255, 255, 10))
Show("xneg", FORMAT$("%x|%X", -1, -255))
Show("cut", FORMAT$("%d|%d|%d|%d|%x", 3.7, -3.7, 2.5, 3.5, 255.9))
Show("32bit", FORMAT$("%d|%d|%d|%d", 2147483647, 2147483648, -2147483648, -2147483649))
Show("32bitx", FORMAT$("%x|%x|%x|%x", 2147483648, 4294967295, 4294967296, 1E10))
d = 1E19
Show("huge", FORMAT$("%d|%x", d, d))
Show("prec15", FORMAT$("%.15d|%.16d|%.15x|%.16x", 5, 5, 255, 255))
Show("zero", FORMAT$("%.0d|%.0x|%d|%x", 0, 0, 0, 0))
' text where a number goes, a number where text goes
Show("strnum", FORMAT$("%d|%f|%x|%e|%g|%n|%m", s, s, s, s, s, s, s))
Show("varstr", FORMAT$("%d|%f|%s", v, v, v))
Show("numstr", FORMAT$("%s|%s|%s", n, 2.5, 1 = 1))
Show("str", FORMAT$("%s|%8s|%-8s|%.2s|%8.3s|%S", s, s, s, s, s, s))
' floating point: RapidQ's digits
Show("f", FORMAT$("%f|%.2f|%10.3f|%-10.3f|%010.3f|%.0f|%.1f", 3.14159, 3.14159, 3.14159, 3.14159, 3.14159, 2.5, 2.25))
Show("round", FORMAT$("%.0f|%.0f|%.0f|%.1f|%.1f|%.2f|%.2f", 0.5, 1.5, 2.5, 0.25, 0.35, 0.125, 0.375))
Show("binary", FORMAT$("%.2f|%.2f|%.2f|%.2f|%.2f|%.2f", 2.675, 1.005, 0.285, 1.15, -2.675, 0.045))
Show("digits", FORMAT$("%.9f|%.9f|%.15f", 1234567890.1, 123456789.125, 99999.9999999999))
Show("long", FORMAT$("%.16f|%.17f|%.18f", 2.675, 2.675, 2.675))
Show("cap", FORMAT$("%.19f|%.20f|%.30f|%.99f", 2.675, 0.5, 1.5, 1234.5))
Show("third", FORMAT$("%.15f|%.17f|%.18f", 1 / 3, 1 / 3, 2 / 3))
d = 99999.9999999999
Show("near", FORMAT$("%.2f|%f", d, d))
Show("small", FORMAT$("%.2f|%f|%.9f|%.2f", 0.00001, 1E-10, 0.000001, 1000000))
Show("big", FORMAT$("%.2f|%f", 1E15, 1E15))
' exponent and general
Show("e", FORMAT$("%e|%.2e|%.0e|%.1e|%.5e|%.18e|%.19e", 1234.5, 1234.5, 1234.5, 1234.5, 1234.5, 1234.5, 1234.5))
Show("e2", FORMAT$("%e|%e|%.3e|%.0e", 0, -0.04, 0.00001, 2.675))
Show("g", FORMAT$("%g|%g|%g|%g|%g|%g", 1234.5, 0.5, 0.00001234, 100000000, 1E15, 0.0001))
Show("gp", FORMAT$("%.3g|%.3g|%.0g|%.1g|%.2g|%.18g|%.19g", 2.675, 1.005, 1234.5, 1234.5, 1234.5, 2.675, 1234.5))
Show("n", FORMAT$("%n|%n|%.0n|%.1n|%.19n|%.18n", 1234567.891, -1234.5, 1234.5, 0.55, 1234.5, 0.5))
Show("m", FORMAT$("%m|%m|%.0m|%.1m|%.19m", 1234.5, -2, 1234.5, 0.55, 1234.5))
Show("m0", FORMAT$("%m|%.0m|%.0m|%.2m|%.2m", 0, -0.4, -0.5, -0.004, -0.0000001))
' a negative number rounded to zero
Show("negzero", FORMAT$("%.1f|%.2f|%.0f|%.0f|%.0f|%.1f|%.2f", -0.04, -0.004, -0.4, -0.04, -0.004, -0.004, -0.0000001))
Show("negzeron", FORMAT$("%.0n|%.2n|%.0n", -0.4, -0.004, -0.04))
' infinities and NaN
d = 1 / z
Show("inf", FORMAT$("%f|%e|%g|%n|%m|%d|%x|%s", d, d, d, d, d, d, d, d))
d = z / z
Show("nan", FORMAT$("%f|%e|%g|%n|%m|%d|%x", d, d, d, d, d, d, d))
d = -1 / z
Show("-inf", FORMAT$("%f|%e|%g", d, d, d))
' index, star, percent
Show("index", FORMAT$("%1:d|%0:d", 5, 6))
s2 = "two"
s3 = "three"
Show("index2", FORMAT$("%2:s-%1:s-%0:s|%0:s", s, s2, s3))
Show("star", FORMAT$("%*d|%-*d|", 5, 42, 6, 42))
Show("pct", FORMAT$("100%%|%d%%|a%%b", 5))
Show("trail", FORMAT$("abc%", 1))
Show("trail2", FORMAT$("%d%", 5))
Show("dot", FORMAT$("%.", 5))
Show("minus", FORMAT$("%-", 5))
Show("extra", FORMAT$("%d", 1, 2, 3))
Show("lit", FORMAT$("a%db%sc", 7, "x"))
Show("idioms", FORMAT$("%s has %d items costing %.2f (%d%%)", s, n, 3.14159, 50))
Show("pad", FORMAT$("[%8.2f]|[%-8.2f]|[%5d]|[%-5d]", 1234.5, 1234.5, 7, 7))
