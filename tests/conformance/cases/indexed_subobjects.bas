' Indexed sub-objects of components (StatusBar panels, ListView columns):
' SB.Panel(0).Width, and inside CREATE SB the bare Panel(0).Width.

CREATE SB AS QSTATUSBAR
    Panel(0).Caption = "Ready"
    Panel(0).Width = 120
END CREATE
SB.Panel(1).Caption = "Line 1"
PRINT SB.Panel(0).Caption; " "; SB.Panel(0).Width; " "; SB.Panel(1).Caption
