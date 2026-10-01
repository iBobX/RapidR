' INPUT$(n): n keys (from a pipe: n characters), not echoed; when the
' input ends first, what came.
a$ = INPUT$(3)
PRINT "[" + a$ + "]"
PRINT VAL(INPUT$(1)) * 2
b$ = INPUT$(5)
PRINT "[" + b$ + "]"; LEN(b$)
