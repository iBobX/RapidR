' Track bars (horizontal with the focus, vertical, disabled), scroll bars
' (horizontal, vertical, disabled) and gauges (bar, vertical, pie, needle).
CREATE Form AS QFORM
  Caption = "Ranges": Left = 40: Top = 40: Width = 400: Height = 320
  CREATE Tb AS QTRACKBAR
    Left = 10: Top = 10: Width = 200: Height = 40: Position = 3
  END CREATE
  CREATE TbOff AS QTRACKBAR
    Left = 10: Top = 56: Width = 200: Height = 40: Position = 7: Enabled = 0
  END CREATE
  CREATE Tv AS QTRACKBAR
    Orientation = 1: Left = 220: Top = 10: Width = 40: Height = 130: Position = 4
  END CREATE
  CREATE Sh AS QSCROLLBAR
    Left = 10: Top = 104: Width = 200: Height = 17: Position = 30
  END CREATE
  CREATE ShOff AS QSCROLLBAR
    Left = 10: Top = 126: Width = 200: Height = 17: Enabled = 0
  END CREATE
  CREATE Sv AS QSCROLLBAR
    Kind = 1: Left = 270: Top = 10: Width = 17: Height = 130: Position = 60
  END CREATE
  CREATE G1 AS QGAUGE
    Left = 10: Top = 152: Width = 200: Height = 20: Position = 40
  END CREATE
  CREATE G2 AS QGAUGE
    Kind = 2: Left = 300: Top = 10: Width = 24: Height = 130: Position = 70
  END CREATE
  CREATE G3 AS QGAUGE
    Left = 10: Top = 180: Width = 70: Height = 70: Kind = 3: Position = 30
  END CREATE
  CREATE G4 AS QGAUGE
    Left = 90: Top = 180: Width = 100: Height = 70: Kind = 4: Position = 60
  END CREATE
  CREATE G5 AS QGAUGE
    Left = 200: Top = 152: Width = 180: Height = 20: Position = 75: ForeColor = &HFF0000: BackColor = &HFFFFFF
  END CREATE
END CREATE
Form.ShowModal
