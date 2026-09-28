' ON ERROR is accepted (VB code) and ignored: nothing here fails at run time
FUNCTION F(n AS INTEGER) AS INTEGER
  ON ERROR RESUME NEXT
  F = n * 2
END FUNCTION
ON ERROR GOTO 0
PRINT F(4)
