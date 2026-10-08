' RapidR Studio's designer is WYSIWYG (docs/ide-plan.md I4, L-DVIEW): a form
' with most of RapidQ's visual components, their design-time properties
' (captions, texts, items, columns, cells, tabs, panels, colours, fonts,
' borders, check states) and containers holding components, a main menu and
' non-visual components for the tray. tools/visual/designer_wysiwyg.py draws
' it in the designer and compares it, pixel for pixel, with the running form.
$INCLUDE "RAPIDQ.INC"

CREATE Form AS QFORM
  Caption = "Designer: every component"
  Width = 640: Height = 520
  CREATE MainMenu AS QMAINMENU
    CREATE FileMenu AS QMENUITEM
      Caption = "&File"
      CREATE OpenItem AS QMENUITEM
        Caption = "&Open...": ShortCut = "Ctrl+O"
      END CREATE
    END CREATE
    CREATE EditMenu AS QMENUITEM
      Caption = "&Edit"
    END CREATE
    CREATE HelpMenu AS QMENUITEM
      Caption = "&Help"
    END CREATE
  END CREATE
  CREATE Ticker AS QTIMER
    Interval = 500
  END CREATE
  CREATE OpenDlg AS QOPENDIALOG
    Filter = "Programs|*.bas"
  END CREATE
  CREATE Images AS QIMAGELIST
  END CREATE
  CREATE NameLbl AS QLABEL
    Caption = "&Name:": Left = 8: Top = 12
  END CREATE
  CREATE NameEd AS QEDIT
    Text = "RapidR": Left = 60: Top = 8: Width = 160
  END CREATE
  CREATE Ok AS QBUTTON
    Caption = "OK": Left = 228: Top = 7: Default = 1
  END CREATE
  CREATE Cancel AS QBUTTON
    Kind = 2: Left = 308: Top = 7
  END CREATE
  CREATE Bold AS QLABEL
    Caption = "Bold red Arial 12": Left = 392: Top = 10
    Font.Name = "Arial": Font.Size = 12: Font.Bold = 1: Font.Color = &H0000FF
  END CREATE
  CREATE Check AS QCHECKBOX
    Caption = "Checked": Left = 8: Top = 40: Checked = 1
  END CREATE
  CREATE Check2 AS QCHECKBOX
    Caption = "Disabled": Left = 100: Top = 40: Enabled = 0
  END CREATE
  CREATE Group AS QGROUPBOX
    Caption = "Size": Left = 8: Top = 66: Width = 180: Height = 80
    CREATE Small AS QRADIOBUTTON
      Caption = "Small": Left = 10: Top = 20: Checked = 1
    END CREATE
    CREATE Large AS QRADIOBUTTON
      Caption = "Large": Left = 10: Top = 46
    END CREATE
  END CREATE
  CREATE Combo AS QCOMBOBOX
    Left = 196: Top = 40: Width = 150
    AddItems "Arial", "Courier New", "Times New Roman"
    ItemIndex = 1
  END CREATE
  CREATE List AS QLISTBOX
    Left = 196: Top = 68: Width = 150: Height = 78
    AddItems "Apples", "Bananas", "Cherries", "Dates", "Elderberries", "Figs"
    ItemIndex = 2
  END CREATE
  CREATE Panel AS QPANEL
    Left = 354: Top = 40: Width = 270: Height = 106
    Color = &HE0FFE0: BevelOuter = 2
    CREATE PanelLbl AS QLABEL
      Caption = "A panel holds components": Left = 8: Top = 8
    END CREATE
    CREATE Cool AS QCOOLBTN
      Caption = "Cool": Left = 8: Top = 30: Width = 60: Height = 24
    END CREATE
    CREATE Oval AS QOVALBTN
      Caption = "Oval": Left = 80: Top = 30: Width = 70: Height = 40
    END CREATE
    CREATE Track AS QTRACKBAR
      Left = 8: Top = 70: Width = 250: Height = 30: Position = 3
    END CREATE
  END CREATE
  CREATE Grid AS QSTRINGGRID
    Left = 8: Top = 154: Width = 300: Height = 110
    ColCount = 4: RowCount = 5
    Cell(1, 0) = "Name": Cell(2, 0) = "Age": Cell(3, 0) = "City"
    Cell(1, 1) = "Ana": Cell(2, 1) = "34": Cell(3, 1) = "Lima"
    ColWidths(0) = 30
  END CREATE
  CREATE View AS QLISTVIEW
    Left = 316: Top = 154: Width = 308: Height = 110
    ViewStyle = 3
    AddColumns "File", "Size"
    Column(0).Width = 180
  END CREATE
  CREATE Tabs AS QTABCONTROL
    Left = 8: Top = 272: Width = 300: Height = 120
    AddTabs "General", "View", "Help"
    CREATE Notes AS QMEMO
      Left = 10: Top = 32: Width = 278: Height = 50
      Text = "Notes on the first page"
    END CREATE
    CREATE Apply AS QBUTTON
      Caption = "&Apply": Left = 10: Top = 88: Width = 80
    END CREATE
  END CREATE
  CREATE Tree AS QTREEVIEW
    Left = 316: Top = 272: Width = 150: Height = 120
    AddItems "North", "South"
    AddChildItems 0, "Hill", "Lake"
  END CREATE
  CREATE Box AS QSCROLLBOX
    Left = 474: Top = 272: Width = 150: Height = 120
    CREATE Far AS QBUTTON
      Caption = "Far away": Left = 160: Top = 140
    END CREATE
    CREATE Near AS QLABEL
      Caption = "Scroll me": Left = 8: Top = 8
    END CREATE
  END CREATE
  CREATE Gauge AS QGAUGE
    Left = 8: Top = 400: Width = 200: Height = 20: Position = 40
  END CREATE
  CREATE Rich AS QRICHEDIT
    Left = 216: Top = 400: Width = 200: Height = 40
    Text = "Rich text"
  END CREATE
  CREATE Bar AS QSCROLLBAR
    Left = 424: Top = 400: Width = 200: Height = 17
    Position = 30
  END CREATE
  CREATE Status AS QSTATUSBAR
    AddPanels "Ready", "Line 1"
    Panel(0).Width = 150
  END CREATE
END CREATE

Form.ShowModal
