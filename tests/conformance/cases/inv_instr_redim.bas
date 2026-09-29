' INV (modular inverse operator), INSTR with an empty start, REDIM keeping data
PRINT 3 INV 26; 7 INV 10; 2 INV 4
PRINT INSTR(, "abcabc", "c"); INSTR(4, "abcabc", "c")
DIM a(3) AS INTEGER
a(2) = 7
REDIM a(5) AS INTEGER
PRINT a(2); UBOUND(a)
