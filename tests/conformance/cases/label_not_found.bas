' A GOTO/GOSUB to a label that doesn't exist in the routine is a compile error, with its position.
PRINT "start"
GOTO Nowhere
SUB Other()
There:
END SUB
GOSUB There
