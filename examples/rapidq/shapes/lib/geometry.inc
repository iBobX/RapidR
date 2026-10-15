' geometry.inc: the areas of three shapes (the Shapes example's library).
' Its own $INCLUDE names the file from the program's folder, as RapidQ
' looks for every include there first.
$INCLUDE "Lib\Units.inc"

CONST PI_VALUE = 3.14159265358979

FUNCTION CircleArea (Radius AS DOUBLE) AS DOUBLE
    Result = PI_VALUE * Radius * Radius
END FUNCTION

FUNCTION SquareArea (Side AS DOUBLE) AS DOUBLE
    Result = Side * Side
END FUNCTION

' (an equilateral triangle)
FUNCTION TriangleArea (Side AS DOUBLE) AS DOUBLE
    Result = SQR(3) / 4 * Side * Side
END FUNCTION
