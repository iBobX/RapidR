' GOSUB and GOTO inside SELECT CASE (also nested in IF), in both backends
DIM total AS INTEGER
DIM k AS INTEGER

FOR k = 1 TO 6
  SELECT CASE k
    CASE 1
      GOSUB AddOne
    CASE 2, 3
      IF k = 3 THEN GOSUB AddTen ELSE GOSUB AddOne
    CASE 4 TO 5
      total = total + 100
      GOSUB AddOne
    CASE ELSE
      GOSUB AddTen
      GOTO Skip
  END SELECT
  PRINT "k="; k; " total="; total
  Skip:
NEXT
PRINT "done"; total
END

AddOne:
  total = total + 1
  RETURN

AddTen:
  total = total + 10
  RETURN
