' EVENT blocks in TYPE … EXTENDS QBUTTON (one This per instance) and a Sender parameter.
$INCLUDE "RAPIDQ.INC"
TYPE TCountButton EXTENDS QBUTTON
  Clicks AS INTEGER
  EVENT OnClick
    Clicks = Clicks + 1
    Caption = "Clicked " + STR$(Clicks)
  END EVENT
  CONSTRUCTOR
    Caption = "Click me"
    Width = 140
  END CONSTRUCTOR
END TYPE

CREATE Form AS QFORM
  Caption = "RapidQ-style OOP"
  Width = 480
  Height = 160
  Center
END CREATE

DIM B1 AS TCountButton
B1.Parent = Form
B1.Left = 10
B1.Top = 10
DIM B2 AS TCountButton
B2.Parent = Form
B2.Left = 160
B2.Top = 10

CREATE B3 AS QBUTTON
  Parent = Form
  Left = 310
  Top = 10
  Width = 150
  Caption = "Sender test"
  OnClick = ShowSender
END CREATE

SUB ShowSender (Sender AS QBUTTON)
  Sender.Caption = "Sender works"
END SUB

Form.ShowModal
