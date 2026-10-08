' A program's own CONST True / False (RAPIDQ.INC's `CONST True = 1`) is what TRUE / FALSE read, natively as in the VM
$APPTYPE CONSOLE
CONST True = 1
CONST False = 0
DIM c AS INTEGER
c = 1
PRINT TRUE; " "; FALSE
IF c = TRUE THEN PRINT "yes"
IF FALSE THEN PRINT "bad"
PRINT "end"
