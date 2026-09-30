' $OPTION BYREF: parameters without BYVAL are passed by reference from there on (RapidQ's default is BYVAL).
SUB Twice (n AS INTEGER)
  n = n * 2
END SUB
$OPTION BYREF
SUB Thrice (n AS INTEGER)
  n = n * 3
END SUB
SUB Kept (BYVAL n AS INTEGER)
  n = 0
END SUB
DIM a AS INTEGER
a = 5
Twice a
PRINT a
Thrice a
PRINT a
Kept a
PRINT a
