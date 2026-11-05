' The interpreter's fused instructions (docs/vm-fast-design.md) on typed
' variables, locals and globals: a variable's or a constant's operand fused
' into + - * / MOD, a typed store without its own ToNum. RapidQ's rules at
' the edges: 32-bit INTEGERs (beyond: -2147483648), the narrow types
' wrapping, DWORD signed, `\` and MOD rounding their operands, -0, NaN,
' infinities, INTEGER and DOUBLE mixed. The answers are the ones the
' interpreter gave before the fused instructions (2.122), and native builds'.
DIM gi AS INTEGER, gj AS INTEGER, gd AS DOUBLE, ge AS DOUBLE, gb AS BYTE, gw AS WORD, gs AS SHORT, gq AS DWORD
DIM gsi AS SINGLE, g64 AS INT64

SUB Locals
  DIM i AS INTEGER, j AS INTEGER, d AS DOUBLE, e AS DOUBLE, b AS BYTE, w AS WORD, s AS SHORT, q AS DWORD
  DIM si AS SINGLE, k AS LONG
  i = 2147483647: j = 1
  i = i + j: PRINT "i+j "; i
  i = 2147483647: i = i + 1: PRINT "i+1 "; i
  i = -2147483647: i = i - 2: PRINT "i-2 "; i
  i = 65536: i = i * i: PRINT "i*i "; i
  i = 46341: j = 46341: k = i * j: PRINT "k "; k
  i = 7: j = 2: PRINT "div "; i / j; " "; i \ j; " "; i MOD j; " "; -i \ j; " "; -i MOD j
  d = 7.5: e = 2: PRINT "dbl "; d \ e; " "; d MOD e; " "; 6.5 \ e; " "; d MOD 4
  d = 2.5: i = d: PRINT "trunc "; i; " "; -d
  d = -2.7: i = d: PRINT "trunc- "; i
  d = 1E10: i = d: PRINT "big "; i
  b = 250: b = b + 10: PRINT "byte "; b
  b = 3: b = b - 5: PRINT "byte- "; b
  w = 65535: w = w + 2: PRINT "word "; w
  s = 32767: s = s + 1: PRINT "short "; s
  q = -1: q = q * 3: PRINT "dword "; q
  si = 0.1: si = si * 3: PRINT "single "; si
  d = 0: e = -1: d = d * e: PRINT "negzero "; d; " "; 1 / d
  d = 1E308: d = d * 10: PRINT "inf "; d; " "; -d
  d = 0: e = 0: d = d / e
  IF d = d THEN
    PRINT "nan = nan"
  ELSE
    PRINT "nan <> nan"
  END IF
  IF d < 1 THEN
    PRINT "nan < 1"
  ELSE
    PRINT "not nan < 1"
  END IF
  IF d > 1 THEN
    PRINT "nan > 1"
  ELSE
    PRINT "not nan > 1"
  END IF
  IF d >= d THEN
    PRINT "nan >= nan"
  ELSE
    PRINT "not nan >= nan"
  END IF
  IF d <> 0 THEN
    PRINT "nan <> 0"
  ELSE
    PRINT "not nan <> 0"
  END IF
  i = 3: d = 0.5: d = i + d: PRINT "mixed "; d; " "; i * d; " "; d - i; " "; i / 4
  i = 9007199254740993# : PRINT "i53 "; i
  e = 2.5: d = e * 2 + e / 2 - 1: PRINT "expr "; d
  d = 1: FOR i = 1 TO 20: d = d * 1.5 + 0.25: NEXT: PRINT "loop "; d
END SUB

Locals
gi = 2147483647: gj = 1
gi = gi + gj: PRINT "gi+gj "; gi
gi = 65536: gi = gi * gi: PRINT "gi*gi "; gi
gi = 7: gj = 2: PRINT "gdiv "; gi / gj; " "; gi \ gj; " "; gi MOD gj; " "; gi MOD 3
gd = 7.5: ge = 2: PRINT "gdbl "; gd \ ge; " "; gd MOD ge; " "; gd * ge - gi
gb = 250: gb = gb + 10: PRINT "gbyte "; gb
gw = 65535: gw = gw + 2: PRINT "gword "; gw
gs = -32768: gs = gs - 1: PRINT "gshort "; gs
gq = 2147483647: gq = gq + 1: PRINT "gdword "; gq
gsi = 1.1: gsi = gsi + 2: PRINT "gsingle "; gsi
g64 = 9223372036854775807: g64 = g64 + 1: PRINT "g64 "; g64
gd = 0: ge = -1: gd = gd * ge: PRINT "gnegzero "; gd
gd = 0: gd = gd / gd
IF gd < 0 THEN
  PRINT "gnan < 0"
ELSE
  PRINT "not gnan < 0"
END IF
IF gd = 5 THEN
  PRINT "gnan = 5"
ELSE
  PRINT "not gnan = 5"
END IF
gd = 0.1: FOR gi = 1 TO 10: gd = gd * 1.1 + 0.01: NEXT: PRINT "gloop "; gd
gi = 5: gd = gi + 0.5: PRINT "gmixed "; gd; " "; gi * 2.5
