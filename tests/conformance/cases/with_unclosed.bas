' A WITH left open is closed by END SUB / END FUNCTION (as RapidQ allows)
TYPE P
  X AS INTEGER
  Y AS INTEGER
END TYPE
DIM pt AS P

SUB Fill (q AS P)
  WITH q
    .X = 3
    .Y = .X + 4
END SUB

FUNCTION Sum (q AS P) AS INTEGER
  WITH q
    Sum = .X + .Y
END FUNCTION

Fill pt
PRINT pt.X, pt.Y
PRINT Sum(pt)
WITH pt
  .X = 10
END WITH
PRINT pt.X
