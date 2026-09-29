' RapidQ programs' looser syntax: a keyword as a variable (`type = 2`) or
' parameter (`case`, with SELECT CASE still working), `ByVal` in a call, `_`
' stuck to a name as a line continuation, comment lines inside a continued
' statement.
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
