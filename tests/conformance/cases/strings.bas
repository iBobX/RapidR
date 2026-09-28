' Core string builtins.
PRINT LEFT$("hello", 2)
PRINT MID$("hello", 2, 3)
PRINT RIGHT$("hello", 3)
PRINT UCASE$("abc") + LCASE$("DEF")
PRINT "[" + LTRIM$("  x ") + "]" + "[" + RTRIM$(" y  ") + "]"
PRINT STR$(LEN("hello"))
PRINT STR$(INSTR("hello", "l"))
PRINT CHR$(65) + STR$(ASC("B"))
PRINT HEX$(255)
PRINT STRING$(3, "*") + "[" + SPACE$(2) + "]"
PRINT "mid2 "; MID$("hello", 2); "|"; MID$("hello", 9); "|"
