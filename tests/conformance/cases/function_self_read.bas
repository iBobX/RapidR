' Inside a FUNCTION with parameters its name read without arguments is a
' call missing them: RC.EXE stops with "Expected ( but got "+"" (RESULT reads
' the result).
FUNCTION F(x AS INTEGER) AS INTEGER
  F = 1
  F = F + x
END FUNCTION
PRINT F(2)
