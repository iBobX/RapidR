' updown.bas for RapidQ's RC.EXE: Windows' up-down controls
' (msctls_updown32, what QUpDown.inc wraps) made directly.
DECLARE FUNCTION CreateWindowEx LIB "user32" ALIAS "CreateWindowExA" (ex AS LONG, cls AS STRING, title AS STRING, style AS LONG, x AS LONG, y AS LONG, w AS LONG, h AS LONG, parent AS LONG, menu AS LONG, inst AS LONG, param AS LONG) AS LONG
DECLARE SUB InitCommonControls LIB "comctl32" ALIAS "InitCommonControls" ()
CREATE Form AS QFORM
  Caption = "Up-down": Left = 40: Top = 40: Width = 260: Height = 120
  CREATE Ed1 AS QEDIT
    Left = 10: Top = 10: Width = 60: Text = "2"
  END CREATE
END CREATE
DIM h AS LONG
InitCommonControls
h = CreateWindowEx(0, "msctls_updown32", "", &H50000000, 72, 9, 17, 24, Form.Handle, 0, 0, 0)
h = CreateWindowEx(0, "msctls_updown32", "", &H50000000, 100, 9, 40, 24, Form.Handle, 0, 0, 0)
h = CreateWindowEx(0, "msctls_updown32", "", &H50000040, 150, 10, 34, 17, Form.Handle, 0, 0, 0)
h = CreateWindowEx(0, "msctls_updown32", "", &H50000000, 10, 44, 17, 40, Form.Handle, 0, 0, 0)
Form.ShowModal
