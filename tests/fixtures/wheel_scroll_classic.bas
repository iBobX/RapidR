' The mouse wheel over a classic program's QSCROLLBAR moves nothing and
' fires nothing, as RapidQ's: RC.EXE (2026-11-05, RUNTIME-BUGS) — the wheel
' message sent and posted to a vertical and a horizontal bar's window, and
' to the form's, left both at Position 5 with no OnChange or OnScroll. (A
' RapidR program's scroll bar moves: wheel_scroll.) tests/gui_parity_cases.mjs
' ("wheel_scroll_classic").
$INCLUDE "RAPIDQ.INC"
DECLARE SUB VChange
DECLARE SUB VScroll (Code AS BYTE, Pos AS LONG)
DECLARE SUB HChange
DECLARE SUB HScroll (Code AS BYTE, Pos AS LONG)
DIM Trail AS STRING
CREATE Form AS QFORM
  Caption = "wheel probe"
  Width = 320
  Height = 260
  CREATE SV AS QSCROLLBAR
    Kind = sbVertical
    Left = 260
    Top = 10
    Width = 17
    Height = 180
    Min = 0
    Max = 10
    Position = 5
    OnChange = VChange
    OnScroll = VScroll
  END CREATE
  CREATE SH AS QSCROLLBAR
    Left = 10
    Top = 200
    Width = 200
    Min = 0
    Max = 10
    Position = 5
    OnChange = HChange
    OnScroll = HScroll
  END CREATE
  CREATE Lk AS QLABEL
    Left = 10
    Top = 10
    Width = 200
    Caption = "-"
  END CREATE
END CREATE
SUB VChange
  Trail = Trail + " v-change"
  Lk.Caption = Trail
END SUB
SUB VScroll (Code AS BYTE, Pos AS LONG)
  Trail = Trail + " v-scroll"
  Lk.Caption = Trail
END SUB
SUB HChange
  Trail = Trail + " h-change"
  Lk.Caption = Trail
END SUB
SUB HScroll (Code AS BYTE, Pos AS LONG)
  Trail = Trail + " h-scroll"
  Lk.Caption = Trail
END SUB
Form.ShowModal
