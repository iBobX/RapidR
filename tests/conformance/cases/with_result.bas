' WITH blocks (`.Member` refers to the WITH object, nested WITH keeps its
' own) and RapidQ's RESULT = value in FUNCTIONs (manual: RESULT).
TYPE TPoint
    X AS INTEGER
    Y AS INTEGER
END TYPE
DIM P AS TPoint
WITH P
    .X = 3
    .Y = .X * 2
END WITH
PRINT P.X; " "; P.Y
FUNCTION SquareSum (a AS INTEGER, b AS INTEGER) AS INTEGER
    RESULT = a * a
    RESULT = RESULT + b * b
END FUNCTION
PRINT SquareSum(3, 4)
FUNCTION Pick (n AS INTEGER) AS STRING
    Result = "small"
    IF n > 10 THEN Result = "big"
END FUNCTION
PRINT Pick(5); " "; Pick(50)
