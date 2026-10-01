' QTABCONTROL as RapidQ's (Windows' tab control): no pages — OnChange shows
' the program's components; the first tab added is selected, InsertTab
' keeps the selected tab, DelTabs, Tab(i) read and set; the program setting
' TabIndex doesn't fire OnChange, the user's clicks and arrow keys do;
' ClientWidth / ClientHeight are the whole control, an aligned component
' fills the area under the tabs.
DECLARE SUB TabChange
DECLARE SUB Report
CONST alBottom = 2
DIM Log AS STRING
CREATE Form AS QFORM
  Caption = "tabs"
  Width = 360: Height = 320
  CREATE Tab AS QTABCONTROL
    AddTabs "Tab 1", "Tab 2", "Tab 3"
    Width = 340: Height = 200
    OnChange = TabChange
    HotTrack = True
    CREATE Panel0 AS QPANEL
      Top = 40: Left = 5
      Width = Tab.ClientWidth - 10: Height = Tab.ClientHeight - 90
      Caption = "Panel 1"
    END CREATE
    CREATE Panel1 AS QPANEL
      Top = 40: Left = 5
      Width = Tab.ClientWidth - 10: Height = Tab.ClientHeight - 90
      Caption = "Panel 2"
      Visible = False
    END CREATE
    CREATE Ed AS QEDIT
      Align = alBottom
    END CREATE
  END CREATE
  CREATE Lbl AS QLABEL
    Top = 210: Width = 340
  END CREATE
  CREATE Btn AS QBUTTON
    Top = 240
    OnClick = Report
  END CREATE
END CREATE
Lbl.Caption = STR$(Tab.TabIndex) + Tab.Tab(1) + STR$(Tab.ClientWidth) + STR$(Tab.ClientHeight)
Tab.InsertTab 0, "New"
Tab.DelTabs 3
Tab.Tab(0) = "First"
Lbl.Caption = Lbl.Caption + "|" + STR$(Tab.TabIndex) + Tab.Tab(0) + Tab.Tab(2)
Form.ShowModal

SUB TabChange
  Log = Log + STR$(Tab.TabIndex) + Tab.Tab(Tab.TabIndex) + ","
  Panel0.Visible = Tab.TabIndex = 0
  Panel1.Visible = Tab.TabIndex = 1
END SUB

SUB Report
  Tab.TabIndex = 1
  Lbl.Caption = Lbl.Caption + "|" + Log + STR$(Tab.TabIndex) + STR$(Panel1.Visible) + "|" + STR$(Ed.Left) + STR$(Ed.Width) + STR$(Ed.Top + Ed.Height)
END SUB
