' A clock on RapidQ's QDigDisplay.inc (by Peter Molloy), with RapidR's
' own seven-segment bitmaps 32.bmp .. 64.bmp: make them next to it first
' with `python3 tools/make_digit_bitmaps.py <folder>`.
' Build with RAPIDR_INCLUDE_PATH pointing at RapidQ's include folder, from
' this folder (the library loads the bitmaps by name).
$INCLUDE "QDigDisplay.inc"
DECLARE SUB Tick
CREATE Form AS QFORM
  Caption = "QDigDisplay clock"
  Width = 200
  Height = 90
  CREATE Clock AS QDigDisplay
    Left = 0
    Top = 0
  END CREATE
  CREATE T AS QTIMER
    Interval = 1000
    OnTimer = Tick
  END CREATE
END CREATE
SUB Tick
  Clock.Display = TIME$
  Clock.Repaint
END SUB
Tick
Form.ShowModal
