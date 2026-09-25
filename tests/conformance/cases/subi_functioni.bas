' SUBI / FUNCTIONI (RapidQ manual chapter 9): any number of arguments,
' read 1-based and split by type through ParamStr$ / ParamVal and the counts.
DECLARE FUNCTIONI FindMax (...) AS DOUBLE
SUBI TestSUBI (...)
  PRINT "String Parameters: "; ParamStrCount
  FOR I = 1 TO ParamStrCount
    PRINT I; " "; ParamStr$(I)
  NEXT I
  PRINT "Numeric Parameters: "; ParamValCount
  FOR I = 1 TO ParamValCount
    PRINT I; " "; ParamVal(I)
  NEXT I
END SUBI
TestSUBI "Hello", 1234, "Hmmm", "Yeah...", 9876, 1*2*3*4, UCASE$("Last one")
FUNCTIONI FindMax (...) AS DOUBLE
  DIM Largest AS DOUBLE
  DIM I AS BYTE
  Largest = -999999999999
  FOR I = 1 TO ParamValCount
    IF ParamVal(I) > Largest THEN
      Largest = ParamVal(I)
    END IF
  NEXT
  FindMax = Largest
END FUNCTIONI
PRINT "Largest number is: "; FindMax(523, 12.4, 602, 45, -1200)
PRINT "Largest number is: "; FindMax(523, 12.4, FindMax(602, 45, -1200))
