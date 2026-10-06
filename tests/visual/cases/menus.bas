' A main menu bar and a pop-up menu, opened by a timer: an item, a
' checked one, a disabled one, a separator, a shortcut, a submenu.
' rapidr-env: RAPIDR_MENU=window
DECLARE SUB PopIt
CREATE Form AS QFORM
  Caption = "Menus": Left = 40: Top = 40: Width = 300: Height = 220
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
  CREATE Pop AS QPOPUPMENU
    CREATE PCut AS QMENUITEM
      Caption = "Cu&t": ShortCut = "Ctrl+X"
    END CREATE
    CREATE PCheck AS QMENUITEM
      Caption = "&Word wrap": Checked = 1
    END CREATE
    CREATE POff AS QMENUITEM
      Caption = "&Disabled": Enabled = 0
    END CREATE
    CREATE PSep AS QMENUITEM
      Caption = "-"
    END CREATE
    CREATE PMore AS QMENUITEM
      Caption = "&More"
      CREATE PSub AS QMENUITEM
        Caption = "Inside"
      END CREATE
    END CREATE
  END CREATE
END CREATE
CREATE Tmr AS QTIMER
  Interval = 300: OnTimer = PopIt
END CREATE
SUB PopIt
  Tmr.Enabled = 0
  Pop.Popup(Form.Left + 30, Form.Top + 60)
END SUB
Form.ShowModal
