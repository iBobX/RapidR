' xfail: vm, codegen — MID$/LEFT$/RIGHT$ slice bytes, not characters (ROADMAP correctness)
' String functions must count characters, not bytes.
PRINT MID$("héllo", 2, 1)
PRINT LEFT$("ñandú", 2)
PRINT RIGHT$("ñandú", 2)
PRINT STR$(LEN("ñandú"))
