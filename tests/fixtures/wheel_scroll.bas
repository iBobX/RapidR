' The mouse wheel (RAPIDR-ISSUES #17): over an RScrollBar it moves the bar
' (three SmallChanges a notch, kept between Min and Max) and fires its
' OnChange; over a painted list — labels on a panel — the panel's
' OnMouseWheel (Delta, X, Y, Shift) hears it (Delta 1 a notch up, -1 down;
' X, Y in the panel), even with the mouse on one of its labels; over the
' form's open area, with no handler bound there, nothing happens.
' A RapidR program's (`$OPTION RAPIDR`, as every .rr file is): in a classic
' program the wheel leaves a scroll bar alone, as RapidQ's (wheel_scroll_classic).
' tests/gui_parity_cases.mjs ("wheel_scroll").
$OPTION RAPIDR
DECLARE SUB Changed
DECLARE SUB RowsWheel (Delta AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
DIM Trail AS STRING
CREATE F AS RForm
  Caption = "wheel"
  ClientWidth = 300
  ClientHeight = 200
  CREATE Card AS RPanel
    Left = 10
    Top = 20
    Width = 200
    Height = 150
    OnMouseWheel = RowsWheel
    CREATE Row1 AS RLabel
      Left = 4
      Top = 4
      Caption = "row 1"
    END CREATE
  END CREATE
  CREATE S AS RScrollBar
    Kind = sbVertical
    Min = 0
    Max = 10
    Left = 260
    Top = 20
    Width = 16
    Height = 150
    OnChange = Changed
  END CREATE
  CREATE Lk AS RLabel
    Left = 10
    Top = 178
    Width = 280
    Caption = "-"
  END CREATE
END CREATE
SUB Changed
  Trail = Trail + " pos" + STR$(S.Position)
  Lk.Caption = Trail
END SUB
SUB RowsWheel (Delta AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
  Trail = Trail + " wheel" + STR$(Delta) + "@" + STR$(X) + "," + STR$(Y)
  Lk.Caption = Trail
  S.Position = S.Position - Delta
END SUB
F.ShowModal
