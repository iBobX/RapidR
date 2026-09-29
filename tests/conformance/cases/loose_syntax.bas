' RapidQ programs' looser syntax: a keyword as a variable (`type = 2`) or
' parameter (`case`, with SELECT CASE still working), `ByVal` in a call, `_`
' stuck to a name as a line continuation, comment lines inside a continued
' statement, SUBI / FUNCTIONI closed by END SUB / END FUNCTION, INITARRAY.
SUB Pick(case AS INTEGER)
  SELECT CASE case
    CASE 1: PRINT "one"
    CASE ELSE: PRINT "other"
  END SELECT
END SUB
DECLARE FUNCTION Twice(n AS INTEGER) AS INTEGER
FUNCTION Twice(n AS INTEGER) AS INTEGER
  Result = n * 2
END FUNCTION
type = 2
PRINT type + 1; " "; Twice(ByVal type)
Pick 1
Pick type
x = Twice_
(4)
PRINT x
PRINT "a" + _
      "b" _
      ' comment line
      + "c"
FUNCTIONI Total(...) AS LONG
  FOR I = 1 TO ParamValCount
    Result = Result + ParamVal(I)
  NEXT
END FUNCTION
SUBI Show(...)
  PRINT ParamStr$(1); ParamValCount
END SUB
PRINT Total(1, 2, 3)
Show "n=", 5, 6
DIM IA(1 TO 10) AS INTEGER
INITARRAY(IA, 55, 234, 45, 99)
PRINT IA(1); " "; IA(2); " "; IA(4); " "; IA(5)
SUB Fill
  DIM B(3) AS STRING
  INITARRAY B, "x", "y"
  PRINT B(0); B(1); B(2)
END SUB
Fill
