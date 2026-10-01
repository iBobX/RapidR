' $OPTION DIM / $OPTION DECIMAL (RapidQ manual, chapter 3)
$OPTION DIM INTEGER
$OPTION DECIMAL ","
n = 7 / 2
PRINT n
DIM m
m = 9 / 4
PRINT m
d# = 7 / 2
PRINT d#
FOR i = 1 TO 2
  PRINT i;
NEXT
PRINT
SUB Bump
  k = 5 / 2
END SUB
Bump
PRINT k
PRINT VAL("3,25") * 4
DIM s AS STRING
s = "x"
PRINT s
