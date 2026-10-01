' Keyboard and mouse events with RapidQ's arguments: OnKeyDown / OnKeyUp
' (Key, Shift) with the virtual-key code, OnKeyPress (Key) with the
' character typed, to the focused control — its form gets them first with
' KeyPreview on (all OnKeyDowns, then the OnKeyPresses); OnMouseDown /
' OnMouseUp (Button, X, Y, Shift) and OnMouseMove (X, Y, Shift) in the
' component under the mouse.
DECLARE SUB EdDown (Key AS WORD, Shift AS INTEGER)
DECLARE SUB EdPress (Key AS BYTE)
DECLARE SUB EdUp (Key AS WORD, Shift AS INTEGER)
DECLARE SUB FormDown (Key AS WORD, Shift AS INTEGER)
DECLARE SUB FormPress (Key AS BYTE)
DECLARE SUB CvDown (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
DECLARE SUB CvMove (X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
DECLARE SUB CvUp (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
DECLARE SUB PnDown (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER, Sender AS QPANEL)
DIM Keys AS STRING, Mice AS STRING
CREATE Form AS QFORM
  Caption = "input events"
  Width = 360
  Height = 260
  KeyPreview = 1
  OnKeyDown = FormDown
  OnKeyPress = FormPress
  CREATE Ed AS QEDIT
    Left = 5
    Top = 5
    OnKeyDown = EdDown
    OnKeyPress = EdPress
    OnKeyUp = EdUp
  END CREATE
  CREATE Cv AS QCANVAS
    Left = 5
    Top = 40
    Width = 120
    Height = 80
    OnMouseDown = CvDown
    OnMouseMove = CvMove
    OnMouseUp = CvUp
  END CREATE
  CREATE Pn AS QPANEL
    Left = 140
    Top = 40
    Width = 100
    Height = 80
    OnMouseDown = PnDown
  END CREATE
  CREATE Lk AS QLABEL
    Left = 5
    Top = 140
    Width = 340
    Caption = "-"
  END CREATE
  CREATE Lm AS QLABEL
    Left = 5
    Top = 165
    Width = 340
    Caption = "-"
  END CREATE
END CREATE
SUB EdDown (Key AS WORD, Shift AS INTEGER)
  Keys = Keys + "d" + STR$(Key) + "," + STR$(Shift) + " ": Lk.Caption = Keys
END SUB
SUB EdPress (Key AS BYTE)
  Keys = Keys + "p" + STR$(Key) + " ": Lk.Caption = Keys
END SUB
SUB EdUp (Key AS WORD, Shift AS INTEGER)
  Keys = Keys + "u" + STR$(Key) + " ": Lk.Caption = Keys
END SUB
SUB FormDown (Key AS WORD, Shift AS INTEGER)
  Keys = Keys + "fd" + STR$(Key) + " ": Lk.Caption = Keys
END SUB
SUB FormPress (Key AS BYTE)
  Keys = Keys + "fp" + CHR$(Key) + " ": Lk.Caption = Keys
END SUB
SUB CvDown (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
  Mice = Mice + "down" + STR$(Button) + STR$(X) + STR$(Y) + STR$(Shift) + " ": Lm.Caption = Mice
END SUB
SUB CvMove (X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
  Mice = Mice + "move" + STR$(X) + STR$(Y) + STR$(Shift) + " ": Lm.Caption = Mice
END SUB
SUB CvUp (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
  Mice = Mice + "up" + STR$(Button) + STR$(X) + STR$(Y) + " ": Lm.Caption = Mice
END SUB
SUB PnDown (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER, Sender AS QPANEL)
  Mice = Mice + "panel" + STR$(X) + STR$(Y) + " " + Sender.Caption: Lm.Caption = Mice
END SUB
Form.ShowModal
