' A main menu (the menu bar), a header with sections, and a status bar
' with panels and its size grip.
CREATE Form AS QFORM
  Caption = "Bars": Left = 40: Top = 40: Width = 380: Height = 200
  CREATE Menu AS QMAINMENU
    CREATE MFile AS QMENUITEM
      Caption = "&File"
      CREATE MOpen AS QMENUITEM
        Caption = "&Open"
      END CREATE
    END CREATE
    CREATE MEdit AS QMENUITEM
      Caption = "&Edit"
    END CREATE
    CREATE MHelp AS QMENUITEM
      Caption = "&Help"
    END CREATE
  END CREATE
  CREATE Hdr AS QHEADER
    Left = 10: Top = 10: Width = 300: Height = 20
  END CREATE
  CREATE Lbl AS QLABEL
    Caption = "Body": Left = 10: Top = 50
  END CREATE
  CREATE SB AS QSTATUSBAR
  END CREATE
END CREATE
Hdr.AddSections "Name", "Size", "Date"
Hdr.Sections(0).Width = 120
SB.AddPanels "Ready", "Line 1", "INS"
SB.Panel(0).Width = 150
SB.Panel(1).Width = 100
Form.ShowModal
