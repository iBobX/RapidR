' Inside a CREATE, a bare property is the object's (`Left = (1000 -
' Width) \ 2`, `PRINT ItemCount`) unless the program has a variable of
' that name; labels and GOSUB inside WITH / CREATE (native builds too); an
' undeclared variable only read is RapidQ's DOUBLE 0.
TYPE TP
  X AS INTEGER
END TYPE
DIM P AS TP, Value AS INTEGER
Value = 42
WITH P
  .X = 1
Again:
  .X = .X + 1
  IF .X < 5 THEN GOTO Again
  PRINT .X
END WITH
CREATE L AS QSTRINGLIST
  GOSUB Fill
  PRINT ItemCount; " "; Value
END CREATE
CREATE F AS QFORM
  Width = 300
  Left = (1000 - Width) \ 2
  Caption = "w" + STR$(Width)
  PRINT Left; " "; Caption
END CREATE
PRINT "["; zz; "]"; zz + 1
END
Fill:
  L.AddItems "a", "b"
RETURN
