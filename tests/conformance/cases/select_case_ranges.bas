' SELECT CASE with ranges and IS comparisons.
DIM n AS INTEGER
FOR n = 0 TO 12 STEP 4
  SELECT CASE n
    CASE 1 TO 5
      PRINT STR$(n) + ":low"
    CASE IS > 10
      PRINT STR$(n) + ":high"
    CASE ELSE
      PRINT STR$(n) + ":other"
  END SELECT
NEXT n
