' QSTATUSBAR panels (AddPanels, Panel(i).Caption / Width, PanelCount),
' checked natively and interpreted by tests/native_gui_events.mjs.
DECLARE SUB Clicked
CREATE Form AS QFORM
  Caption = "Status": Width = 400: Height = 150
  CREATE Btn AS QBUTTON
    Caption = "Go": OnClick = Clicked
  END CREATE
  CREATE Lbl AS QLABEL
    Top = 40: Width = 300
  END CREATE
  CREATE SB AS QSTATUSBAR
    AddPanels "Ready", "Line 1"
    Panel(0).Width = 150
  END CREATE
END CREATE
SB.AddPanels "INS"

SUB Clicked
  SB.Panel(1).Caption = "Line 42"
  Lbl.Caption = SB.Panel(0).Caption + "|" + SB.Panel(1).Caption + "|" + SB.Panel(2).Caption + "|" + STR$(SB.PanelCount) + "|" + STR$(SB.Panel(0).Width)
END SUB

Form.ShowModal
