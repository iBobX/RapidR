' User-defined TYPE with fields.
TYPE TPoint
  X AS INTEGER
  Y AS INTEGER
END TYPE
DIM p AS TPoint
p.X = 3
p.Y = 4
PRINT STR$(p.X * p.X + p.Y * p.Y)
