' Routines that differ only by type suffix are different routines (Day$ and Day)
FUNCTION Day$ (k AS INTEGER) AS STRING
  Day$ = "day" + STR$(k)
END FUNCTION

FUNCTION Day (k AS INTEGER) AS INTEGER
  Day = k * 2
END FUNCTION

PRINT Day$(3)
PRINT Day(3)
