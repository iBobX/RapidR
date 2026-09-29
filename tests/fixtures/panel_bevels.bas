' QPANEL bevels (BevelOuter / BevelInner / BevelWidth / BorderWidth, a
' raised outer bevel by default) and a TYPE extending QPANEL — created
' inside the form, its PROPERTY SET storing `.Shape` — as include
' libraries like QBevel.inc do.
TYPE QFramed EXTENDS QPANEL
  WITH QFramed
    Shape AS LONG PROPERTY SET Set_Shape
    PROPERTY SET Set_Shape(NewShape AS LONG)
      .Shape = NewShape
      IF .Shape = 1 THEN
        .BevelInner = 2
        .BevelOuter = 1
      ELSE
        .BevelInner = 0
        .BevelOuter = 0
      END IF
    END PROPERTY
    CONSTRUCTOR
      Set_Shape(0)
    END CONSTRUCTOR
  END WITH
END TYPE
DECLARE SUB Frame
CREATE Form AS QFORM
  Caption = "panel bevels"
  Width = 360
  Height = 200
  CREATE P1 AS QPANEL
    Left = 10 : Top = 10 : Width = 100 : Height = 60
  END CREATE
  CREATE P2 AS QPANEL
    Left = 120 : Top = 10 : Width = 100 : Height = 60
    BevelOuter = 1 : BevelInner = 2 : BevelWidth = 2 : BorderWidth = 3
  END CREATE
  CREATE F AS QFramed
    Left = 230 : Top = 10 : Width = 100 : Height = 60
  END CREATE
  CREATE Btn AS QBUTTON
    Left = 10 : Top = 100
    Caption = "Frame"
    OnClick = Frame
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 10 : Top = 140 : Width = 340
    Caption = "-"
  END CREATE
END CREATE
SUB Frame
  F.Shape = 1
  Lbl.Caption = STR$(P1.BevelOuter) + STR$(P1.BevelInner) + STR$(P1.BevelWidth) + " " + STR$(P2.BevelOuter) + STR$(P2.BevelInner) + STR$(P2.BorderWidth) + " " + STR$(F.Shape) + STR$(F.BevelOuter) + STR$(F.BevelInner)
END SUB
Form.ShowModal
