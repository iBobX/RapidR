' Operator semantics from the RapidQ manual (Appendix C): NOT binds looser
' than comparisons, MOD looser than * and /, "jello" - "l" = "jeo", a NOT= b,
' plus SUBs named like file keywords, array parameters and @var by reference
' (manual 3.5: `StrCat(@A$, " World!")`).
x = 3
IF NOT x = 5 THEN PRINT "not five" ELSE PRINT "five"
PRINT NOT x = 3
PRINT 7 MOD 4 * 2
PRINT 2 * 7 MOD 4
PRINT "jello" - "l"; " "; "banana" - "an"
IF x NOT= 4 THEN PRINT "differs"
SUB Close
  PRINT "my Close"
END SUB
SUB Open (n AS INTEGER)
  PRINT "my Open"; n
END SUB
Close
Open 2
SUB Fill (list() AS STRING, n AS INTEGER)
  FOR i = 0 TO n
    list(i) = "item" + STR$(i)
  NEXT
END SUB
DIM names(2) AS STRING
Fill names, 2
PRINT names(0); " "; names(2)
SUB StrCat (Source AS STRING, Text AS STRING)
    Source = Source + Text
END SUB
A$ = "Hello"
StrCat(@A$, " World!")
PRINT A$
B$ = "Bye"
StrCat(B$, " now")
PRINT B$
FUNCTION Bump (n AS INTEGER) AS INTEGER
    n = n + 1
    Bump = n * 10
END FUNCTION
k = 1
PRINT Bump(@k); " "; k; " "; Bump(k); " "; k
a$ = "left" : b$ = "right"
SWAP a$, b$
PRINT a$; " "; b$
DIM nums(2) AS INTEGER
nums(0) = 1 : nums(2) = 3
SWAP nums(0), nums(2)
PRINT nums(0); nums(2)
PRINT 10 SHL 2; " "; 10 SHR 1; " "; 1 SHL 31; " "; &H80000001 SHL 1; " "; 3 + 1 SHL 2
