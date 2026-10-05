' String functions at their edges as RapidQ has them — the .expected is
' RC.EXE's output (docs/rapidq-ground-truth.md): positions before the start
' or past the end, negative counts, INSTR's start (0 counts as 1; before the
' first character the place found is still counted from it) and its empty
' needle (never found), STRING$ of a real code (rounded half to even),
' CONVBASE$ of a negative number (its 32 bits).
PRINT "a ["; MID$("hello", 0, 2); "] ["; MID$("hello", -1, 2); "] ["; MID$("hello", 10, 2); "] ["; MID$("hello", 2, -1); "] ["; MID$("hello", 4); "]"
PRINT "b ["; LEFT$("abc", -1); "] ["; LEFT$("abc", 5); "] ["; RIGHT$("abc", 5); "] ["; RIGHT$("abc", -1); "] ["; RIGHT$("abc", 0); "]"
PRINT "c ["; SPACE$(-3); "] ["; STRING$(-1, "x"); "] ["; STRING$(3, 65); "] ["; STRING$(2, "xy"); "] ["; STRING$(2, 65.7); "] ["; STRING$(2, 65.5); "]"
PRINT "d "; INSTR(0, "abc", "b"); " "; INSTR(5, "abc", "b"); " "; INSTR("abc", ""); " "; INSTR("", "a"); " "; INSTR(-5, "abc", "b"); " "; INSTR(2, "abcabc", "a")
PRINT "e "; ASC("A"); " "; LEN(""); " ["; CHR$(65.7); "] ["; CHR$(66.5); "]"
PRINT "f ["; UCASE$("aBc1"); "] ["; LCASE$("AbC"); "] ["; LTRIM$("  a "); "] ["; RTRIM$(" a  "); "]"
PRINT "g "; RINSTR("abcabc", "b"); " "; RINSTR("abc", "z"); " "; TALLY("abcabc", "bc"); " "; TALLY("aaa", "aa")
PRINT "h ["; DELETE$("abcdef", 2, 3); "] ["; INSERT$("XY", "abc", 2); "] ["; REVERSE$("abc"); "] ["; REPLACESUBSTR$("aXbXc", "X", "--"); "]"
PRINT "i ["; FIELD$("a,b,c", ",", 2); "] ["; FIELD$("a,b,c", ",", 5); "] ["; FIELD$("a b", " ", 1); "]"
PRINT "j ["; CONVBASE$("255", 10, 16); "] ["; CONVBASE$("FF", 16, 2); "] ["; CONVBASE$("-10", 10, 16); "]"
PRINT "k "; 2 ^ (-1); " "; 10 ^ (-2); " "; 2 ^ 31; " "; 2 ^ 0.5 * 2 ^ 0.5
