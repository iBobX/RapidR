' A QEDIT's AutoSize (True until set) fits its Height to its font when its
' form first shows, as RapidQ's (Windows makes the edit's window then):
' Height 50 set before reads 21 after; one set after stays. The .expected
' is RC.EXE's output.
$APPTYPE CONSOLE
CREATE Form AS QFORM
  Width = 300 : Height = 200
  CREATE E AS QEDIT
    Left = 10 : Top = 10 : Width = 200 : Height = 50
  END CREATE
  CREATE E2 AS QEDIT
    Left = 10 : Top = 80 : Width = 200
  END CREATE
END CREATE
PRINT E.Height; " "; E2.Height; " "; E.AutoSize
Form.Show
DOEVENTS
PRINT E.Height; " "; E2.Height
E.Height = 60
PRINT E.Height
E.AutoSize = 0
E.Height = 60
PRINT E.Height
Form.Close
