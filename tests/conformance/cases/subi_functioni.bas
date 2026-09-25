' SUBI / FUNCTIONI (RapidQ manual chapter 9): any number of arguments,
' read 1-based and split by type through ParamStr$ / ParamVal and the counts.
DECLARE FUNCTIONI Total (...) AS DOUBLE
SUBI Inventory (...)
  DIM K AS INTEGER
  PRINT "names"; ParamStrCount; ":";
  FOR K = 1 TO ParamStrCount
    PRINT " ["; ParamStr$(K); "]";
  NEXT K
  PRINT
  PRINT "amounts"; ParamValCount; ":";
  FOR K = 1 TO ParamValCount
    PRINT " "; ParamVal(K);
  NEXT K
  PRINT
END SUBI
Inventory "pens", 12, LCASE$("CUPS"), 7 + 8, "ink" + "jet", 2 ^ 5, "", -3.5
FUNCTIONI Total (...) AS DOUBLE
  DIM Sum AS DOUBLE
  DIM N AS BYTE
  Sum = 0
  FOR N = ParamValCount TO 1 STEP -1
    Sum = Sum + ParamVal(N)
  NEXT
  Total = Sum
END FUNCTIONI
PRINT "total "; Total(1.5, 2.25, 40)
PRINT "nested "; Total(10, Total(1, 2, 3), "skipped", -4)
