' An object field's own properties (`P.MoverRect.Top`, `.MoverRect.Top` in
' the TYPE's code), as RapidQ's include libraries use QRECT fields.
TYPE QSplit EXTENDS QPANEL
  MoverRect AS QRECT
  SUB Place
    QSplit.MoverRect.Top = 5
    PRINT QSplit.MoverRect.Top + 1
    WITH QSplit
      PRINT .MoverRect.Top * 2
    END WITH
  END SUB
END TYPE
CREATE P AS QSplit
END CREATE
P.Place
PRINT P.MoverRect.Top
