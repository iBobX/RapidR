' Extreme values never crash a program (both backends): integer overflow
' wraps (MIN \ -1, MIN MOD -1, -MIN), string functions clamp bad
' positions/lengths, CHR$ of an out-of-range code, INV of extremes, and
' STRING$ with a character code. (`\` and MOD by zero stop a program, as in
' RapidQ: rapidq_division_by_zero.)
DIM a AS VARIANT  ' (RapidR's 64-bit integers; undeclared, or DIMmed without AS, a DOUBLE)
a = -9223372036854775807 - 1
PRINT a \ -1; " "; a MOD -1; " "; -a
PRINT MID$("hello", -5, 3); "|"; LEFT$("abc", -2); "|"; RIGHT$("abc", -1); "|"; SPACE$(-3); "|"
PRINT STRING$(-1, "x"); "|"; STRING$(3, 65); STRING$(2, "xy"); "|"; INSTR(-5, "abc", "b")
PRINT INV(a, -1); " "; INV(3, 7)
