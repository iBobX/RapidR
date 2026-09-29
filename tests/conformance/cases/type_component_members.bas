' A TYPE extending a component (as RapidQ's include libraries write them):
' a field redeclaring one of the component's properties is that property;
' a FOR counter in its code is a local; `TypeName.X` in a PROPERTY SET is
' the instance's field; its instance created inside a form works.
TYPE QDigits EXTENDS QCANVAS
  PUBLIC:
    Width AS LONG
    Height AS LONG
    Display AS STRING
    Shape AS LONG PROPERTY SET Set_Shape
  PROPERTY SET Set_Shape(N AS LONG)
    QDigits.Shape = N * 2
  END PROPERTY
  FUNCTION Sum AS LONG
    T = 0
    FOR L = 1 TO LEN(QDigits.Display)
      T = T + VAL(MID$(QDigits.Display, L, 1))
    NEXT
    Result = T * 100 + L
  END FUNCTION
  CONSTRUCTOR
    Display = "0"
  END CONSTRUCTOR
END TYPE
CREATE Form AS QFORM
  CREATE D AS QDigits
    Width = 50
  END CREATE
END CREATE
D.Display = "1234"
D.Height = 24
D.Shape = 3
PRINT D.Width; " "; D.Height; " "; D.Shape; " "; D.Sum
