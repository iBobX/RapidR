' A stray END STRUCT / END TYPE (no TYPE open) is RC.EXE's END: the program ends there, in a SUB too (checked against RC.EXE; RapidQ's Network/Download/qdownload.bas ends with one)
STRUCT T
  x AS INTEGER
END STRUCT
DIM v AS T
v.x = 3
PRINT v.x
SUB S
  PRINT "in S"
  END STRUCT
  PRINT "not after END STRUCT"
END SUB
PRINT "before"
IF v.x = 4 THEN END TYPE
PRINT "after END TYPE not taken"
S
PRINT "not after S"
end struct
