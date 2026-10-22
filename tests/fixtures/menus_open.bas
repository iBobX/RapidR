' An open menu for the accessibility parity test (tests/web_a11y.mjs): the
' script presses Alt+F, Down four times and Right, so File's menu is down
' with its Recent submenu open — a menu node with its items (a check item,
' radio items, a disabled one, a ShortCut, a submenu) in each host's
' accessibility tree.
CREATE Form AS QFORM
  Caption = "menus open"
  CREATE MainMenu AS QMAINMENU
    CREATE FileMenu AS QMENUITEM
      Caption = "&File"
      CREATE NewItem AS QMENUITEM
        Caption = "&New"
        ShortCut = "Ctrl+N"
      END CREATE
      CREATE Sep AS QMENUITEM
        Caption = "-"
      END CREATE
      CREATE Beg AS QMENUITEM
        Caption = "&Beginner"
        RadioItem = 1
        Checked = 1
      END CREATE
      CREATE Expert AS QMENUITEM
        Caption = "&Expert"
        RadioItem = 1
      END CREATE
      CREATE WrapItem AS QMENUITEM
        Caption = "&Word wrap"
        Checked = 1
      END CREATE
      CREATE RecentMenu AS QMENUITEM
        Caption = "&Recent"
        CREATE OneItem AS QMENUITEM
          Caption = "&1  notes.rr"
          Hint = "/home/user/notes.rr"
        END CREATE
        CREATE TwoItem AS QMENUITEM
          Caption = "&2  menus.rr"
        END CREATE
      END CREATE
      CREATE QuitItem AS QMENUITEM
        Caption = "E&xit"
        Enabled = 0
      END CREATE
    END CREATE
    CREATE EditMenu AS QMENUITEM
      Caption = "&Edit"
      CREATE CutItem AS QMENUITEM
        Caption = "Cu&t"
        ShortCut = "Ctrl+X"
      END CREATE
    END CREATE
  END CREATE
  CREATE Lbl AS QLABEL
    Width = 300
    Caption = "ready"
  END CREATE
END CREATE
Form.ShowModal
