' Console statements (RapidQ appendix C) as ANSI escape sequences: CLS,
' COLOR fg[, bg] (QBasic numbers), LOCATE row, col with omitted values,
' CSRLIN / POS(0), bare TIMER, and omitted arguments (INSTR(, a, b)).
CLS
PRINT "top"
COLOR 14, 1
PRINT "yellow on blue"
COLOR
LOCATE 5, 10
PRINT "at 5,10";
PRINT CSRLIN; POS(0)
LOCATE , 3
PRINT "col 3"
COLOR , 2
PRINT "green background"
t = TIMER
PRINT t >= 0; INSTR(, "banana", "an")
