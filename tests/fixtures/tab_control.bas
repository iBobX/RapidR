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
    Width = 340: Height = 200
    HotTrack = True
    AddTabs "Tab 1", "Tab 2", "Tab 3"
    OnChange = TabChange
    ' (two pages the program swaps itself: the tab control holds none)
    CREATE PageA AS QPANEL
      Left = 12: Top = 34: Width = 220: Height = 96
      Caption = "first page"
    END CREATE
    CREATE PageB AS QPANEL
      Left = 12: Top = 34: Width = 220: Height = 96
      Caption = "second page"
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
  PageA.Visible = Tab.TabIndex = 0
  PageB.Visible = Tab.TabIndex = 1
END SUB

SUB Report
  Tab.TabIndex = 1
  Lbl.Caption = Lbl.Caption + "|" + Log + STR$(Tab.TabIndex) + STR$(PageB.Visible) + "|" + STR$(Ed.Left) + STR$(Ed.Width) + STR$(Ed.Top + Ed.Height)
END SUB
