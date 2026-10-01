' QTRACKBAR as RapidQ's manual has it: Max 10, PageSize 2, LineSize 1,
' Frequency 1 and tsAuto by default; Position kept inside Min..Max; the
' arrows move by LineSize, Page Down by PageSize, Home / End to the ends, a
' click beside the thumb a page toward it (OnChange each time the user moves
' it, not when the program sets it); TickMarks, TickStyle and SetTick.
DECLARE SUB Changed
DECLARE SUB Report
DIM Log AS STRING
CREATE Form AS QFORM
  Caption = "trackbar"
  CREATE Tb AS QTRACKBAR
    Width = 200
    OnChange = Changed
  END CREATE
  CREATE Vt AS QTRACKBAR
    Left = 220: Height = 150
    Orientation = 1
    TickMarks = 2
    TickStyle = 2
  END CREATE
  CREATE Lbl AS QLABEL
    Top = 60: Width = 200
  END CREATE
  CREATE Btn AS QBUTTON
    Top = 90
    OnClick = Report
  END CREATE
END CREATE
Vt.SetTick 5
Lbl.Caption = STR$(Tb.Max) + STR$(Tb.PageSize) + STR$(Tb.LineSize) + STR$(Tb.Frequency) + STR$(Tb.TickStyle)
Tb.Position = 25
Lbl.Caption = Lbl.Caption + "|" + STR$(Tb.Position)
Tb.Position = 3
Form.ShowModal

SUB Changed
  Log = Log + STR$(Tb.Position) + ","
END SUB

SUB Report
  Tb.Max = 4
  Lbl.Caption = Lbl.Caption + "|" + Log + "|" + STR$(Tb.Position)
END SUB
