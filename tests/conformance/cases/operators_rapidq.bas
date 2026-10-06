' Operator semantics from the RapidQ manual (Appendix C): NOT binds looser
' than comparisons, MOD looser than * and /, "jello" - "l" = "jeo", a NOT= b,
' plus SUBs named like file keywords, array parameters and @var by reference
' (`@` passes a variable by reference: manual 3.5).
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
SUB Append (Dest AS STRING, Extra AS STRING)
    Dest = Dest + Extra
END SUB
G$ = "Hello"
Append(@G$, " World!")
PRINT G$
B$ = "Bye"
Append(B$, " now")
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
' AND / OR / XOR / NOT are bitwise (manual Appendix C); conditions still work.
PRINT 5 AND 3; " "; 5 OR 3; " "; 5 XOR 3; " "; NOT -1; " "; NOT 0; " "; NOT 5; " "; 4 OR 32
flags = 1 OR 2 OR 8
IF flags AND 8 THEN PRINT "has 8" ELSE PRINT "no 8"
IF (flags AND 4) = 0 AND x < 100 THEN PRINT "no 4, small x"
