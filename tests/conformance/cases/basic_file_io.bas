' BASIC file I/O by file number: OPEN, PRINT #, WRITE #, INPUT #, LINE INPUT #, EOF, LOF, SEEK, CLOSE.
F$ = "tests/conformance/.work/basic_files.txt"
n = FREEFILE
OPEN F$ FOR OUTPUT AS #n
PRINT #n, "Hello", "World"
WRITE #n, "a b", 7, "z"
PRINT #n, "last line"
CLOSE #n
PRINT "free "; FREEFILE
OPEN F$ FOR INPUT AS #1
LINE INPUT #1, a$
PRINT "[" + a$ + "]"
INPUT #1, s$, x, t$
PRINT "[" + s$ + "] "; x * 2; " [" + t$ + "]"
WHILE NOT EOF(1)
    LINE INPUT #1, l$
    PRINT "rest: " + l$
WEND
PRINT "lof "; LOF(1); " eof "; EOF(1)
CLOSE #1
OPEN F$ FOR APPEND AS #2
PRINT #2, "appended"
CLOSE #2
OPEN F$ FOR INPUT AS #3
c = 0
WHILE NOT EOF(3)
    LINE INPUT #3, l$
    c = c + 1
WEND
CLOSE #3
PRINT "lines "; c
OPEN "tests/conformance/.work/no_such_file.txt" FOR INPUT AS #4
PRINT "missing: eof "; EOF(4); " [" + a$ + "]"
KILL F$
