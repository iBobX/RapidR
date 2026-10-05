' Kernel themes (docs/desktop-host-plan.md, "Kernel themes"): one form with
' the main components, which tests/native_gui_events.mjs captures under
' each theme — classic (RapidQ's, every program's unless it asks for
' another), modern, dark and highcontrast (RAPIDR_THEME, as a program
' without $THEME takes it). A button's OnClick switches at run time
' (Application.Theme, RapidR's), and the program reads the theme back;
' another opens a pop-up menu (a checked item, a disabled one, a
' separator, a submenu) to see the menus drawn.
DECLARE SUB DarkClick
DECLARE SUB MenuClick
CREATE Form AS QFORM
  Caption = "Themes": Width = 540: Height = 430
  CREATE MainMenu AS QMAINMENU
    CREATE MenuFile AS QMENUITEM
      Caption = "&File"
      CREATE MenuOpen AS QMENUITEM
        Caption = "&Open...": ShortCut = "Ctrl+O"
      END CREATE
    END CREATE
    CREATE MenuView AS QMENUITEM
      Caption = "&View"
    END CREATE
    CREATE MenuHelp AS QMENUITEM
      Caption = "&Help"
    END CREATE
  END CREATE
  CREATE LblName AS QLABEL
    Caption = "&Name:": Left = 12: Top = 14: Width = 50
  END CREATE
  CREATE EdName AS QEDIT
    Left = 64: Top = 10: Width = 170: Text = "RapidR"
  END CREATE
  CREATE ChkBold AS QCHECKBOX
    Caption = "&Bold": Left = 12: Top = 42: Width = 70: Checked = 1
  END CREATE
  CREATE ChkItalic AS QCHECKBOX
    Caption = "&Italic": Left = 90: Top = 42: Width = 70
  END CREATE
  CREATE ChkOff AS QCHECKBOX
    Caption = "Off": Left = 168: Top = 42: Width = 60: Checked = 1: Enabled = 0
  END CREATE
  CREATE GrpSize AS QGROUPBOX
    Caption = "Size": Left = 12: Top = 68: Width = 222: Height = 52
    CREATE RadSmall AS QRADIOBUTTON
      Caption = "&Small": Left = 10: Top = 22: Width = 64
    END CREATE
    CREATE RadLarge AS QRADIOBUTTON
      Caption = "&Large": Left = 80: Top = 22: Width = 64: Checked = 1
    END CREATE
    CREATE RadHuge AS QRADIOBUTTON
      Caption = "Huge": Left = 150: Top = 22: Width = 64: Enabled = 0
    END CREATE
  END CREATE
  CREATE CmbFont AS QCOMBOBOX
    Left = 12: Top = 130: Width = 222
    AddItems "Arial", "Courier New", "Times New Roman"
    ItemIndex = 0
  END CREATE
  CREATE LstItems AS QLISTBOX
    Left = 12: Top = 160: Width = 222: Height = 96
    AddItems "Apples", "Bananas", "Cherries", "Dates", "Elderberries", "Figs", "Grapes", "Honeydew"
    ItemIndex = 1
  END CREATE
  CREATE Trk AS QTRACKBAR
    Left = 8: Top = 264: Width = 230: Height = 40: Position = 4
  END CREATE
  CREATE Gauge AS QGAUGE
    Left = 12: Top = 310: Width = 222: Height = 18: Position = 60
  END CREATE
  CREATE Tabs AS QTABCONTROL
    Left = 250: Top = 10: Width = 266: Height = 150
    AddTabs "General", "View", "Help"
    CREATE MemNotes AS QMEMO
      Left = 10: Top = 32: Width = 244: Height = 76
      Text = "Old programs keep RapidQ's look." + CHR$(13) + CHR$(10) + "New ones can choose a modern one," + CHR$(13) + CHR$(10) + "a dark one or high contrast." + CHR$(13) + CHR$(10) + "Line 4" + CHR$(13) + CHR$(10) + "Line 5" + CHR$(13) + CHR$(10) + "Line 6"
    END CREATE
    CREATE BtnApply AS QBUTTON
      Caption = "&Apply": Left = 10: Top = 114: Width = 80
    END CREATE
  END CREATE
  CREATE PnlTools AS QPANEL
    Caption = "": Left = 250: Top = 170: Width = 266: Height = 40
    CREATE Cool1 AS QCOOLBTN
      Caption = "Left": Left = 6: Top = 6: Width = 60: Height = 28: GroupIndex = 1: Down = 1
    END CREATE
    CREATE Cool2 AS QCOOLBTN
      Caption = "Right": Left = 70: Top = 6: Width = 60: Height = 28: GroupIndex = 1
    END CREATE
    CREATE Copies AS QUPDOWN
      Left = 230: Top = 6: Width = 28: Height = 28: Position = 2
    END CREATE
  END CREATE
  CREATE LblNote AS QLABEL
    Caption = "A label on the form": Left = 250: Top = 222: Width = 266
  END CREATE
  CREATE LblColored AS QLABEL
    Caption = "The program's colours": Left = 250: Top = 244: Width = 266: Color = &HE0FFFF: Font.Color = &H800000
  END CREATE
  CREATE BtnOK AS QBUTTON
    Caption = "OK": Left = 250: Top = 300: Width = 80: Default = 1
  END CREATE
  CREATE BtnCancel AS QBUTTON
    Caption = "Cancel": Left = 343: Top = 300: Width = 80: Cancel = 1
  END CREATE
  CREATE BtnOff AS QBUTTON
    Caption = "Disabled": Left = 436: Top = 300: Width = 80: Enabled = 0
  END CREATE
  CREATE BtnDark AS QBUTTON
    Caption = "&Dark": Left = 436: Top = 268: Width = 80: OnClick = DarkClick
  END CREATE
  CREATE BtnMenu AS QBUTTON
    Caption = "&Menu": Left = 343: Top = 268: Width = 80: OnClick = MenuClick
  END CREATE
  CREATE PopMenu AS QPOPUPMENU
    CREATE PopCut AS QMENUITEM
      Caption = "Cu&t": ShortCut = "Ctrl+X"
    END CREATE
    CREATE PopCopy AS QMENUITEM
      Caption = "&Copy": ShortCut = "Ctrl+C"
    END CREATE
    CREATE PopPaste AS QMENUITEM
      Caption = "&Paste": Enabled = 0
    END CREATE
    CREATE PopSep AS QMENUITEM
      Caption = "-"
    END CREATE
    CREATE PopWrap AS QMENUITEM
      Caption = "&Word wrap": Checked = 1
    END CREATE
    CREATE PopZoom AS QMENUITEM
      Caption = "&Zoom"
      CREATE PopZoomIn AS QMENUITEM
        Caption = "Zoom &in"
      END CREATE
    END CREATE
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 250: Top = 334: Width = 266
  END CREATE
  CREATE SB AS QSTATUSBAR
    SimpleText = "Ready"
  END CREATE
END CREATE
Lbl.Caption = "theme " + Application.Theme
EdName.SetFocus
Form.ShowModal

SUB MenuClick
  PopMenu.Popup(Form.Left + 260, Form.Top + 150)
END SUB

SUB DarkClick
  Application.Theme = "dark"
  Lbl.Caption = Lbl.Caption + " then " + Application.Theme
END SUB
