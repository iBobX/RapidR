' A component used as an array — `matrix(1) = 5` where `matrix` is a QLABEL —
' RC.EXE stops at the bracket: `Expected = but got "("` (RapidR found out only
' when the program ran: "not an array").
CREATE Form AS QFORM
  CREATE Matrix AS QLABEL
  END CREATE
END CREATE
matrix(1) = 5
PRINT "ok"
