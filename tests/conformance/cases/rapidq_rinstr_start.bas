' RINSTR([start,] s, find) as RapidQ computes it — the .expected is RC.EXE's
' output (2026-10-22, docs/rapidq-ground-truth.md): with a start, only an
' occurrence that ends at or before character `start` counts (the search
' is in LEFT$(s, start)); a start of 0 or less finds nothing, one past the
' end searches all of s; a real start is rounded half to even. (RapidQ
' stops with an access violation on an empty find string; RapidR gives 0,
' as INSTR does.)
PRINT "doc "; RINSTR("Hello World", "l"); " "; RINSTR(4, "hehe they", "he")
PRINT "s "; RINSTR(3, "hehe they", "he"); " "; RINSTR(2, "hehe they", "he"); " "; RINSTR(1, "hehe they", "he")
PRINT "z "; RINSTR(0, "hehe they", "he"); " "; RINSTR(-5, "hehe they", "he")
PRINT "t "; RINSTR(7, "hehe they", "he"); " "; RINSTR(8, "hehe they", "he"); " "; RINSTR(100, "hehe they", "he")
PRINT "aa "; RINSTR(3, "aaaa", "aa"); " "; RINSTR(4, "aaaa", "aa")
PRINT "bc "; RINSTR(4, "abcabc", "bc"); " "; RINSTR(5, "abcabc", "bc"); " "; RINSTR(6, "abcabc", "bc")
PRINT "real "; RINSTR(2.5, "abcabc", "c"); " "; RINSTR(2.7, "abcabc", "c"); " "; RINSTR(3.5, "abcabc", "c")
PRINT "big "; RINSTR(3000000000, "abcabc", "c"); " "; RINSTR(-3000000000, "abcabc", "c")
PRINT "edge "; RINSTR("abc", "abcd"); " "; RINSTR(2, "abc", "abcd"); " "; RINSTR("", "a"); " "; RINSTR("ABCabc", "C")
DIM n AS INTEGER
n = 4
PRINT "var "; RINSTR(n, "hehe they", "he")
