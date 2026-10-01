' QUICKSORT (a range of one array, ASCEND / DESCEND), TAB, ATAN, GET$ (stdin)
DIM A(1 TO 6) AS INTEGER, S(3) AS STRING
A(1) = 5: A(2) = 3: A(3) = 9: A(4) = 1: A(5) = 7: A(6) = 2
QUICKSORT A(2), A(5), ASCEND
FOR i = 1 TO 6: PRINT A(i);" ";: NEXT: PRINT
QUICKSORT(A(1), A(6), DESCEND)
FOR i = 1 TO 6: PRINT A(i);" ";: NEXT: PRINT
S(0) = "pear": S(1) = "Apple": S(2) = "fig": S(3) = "apple"
QUICKSORT S(0), S(3), ASCEND
PRINT S(0); ","; S(1); ","; S(2); ","; S(3)
PRINT "ab"; TAB(6); "x"; TAB(3); "y"
PRINT ATAN(1) * 4
PRINT "[" + GET$(3) + "]"
