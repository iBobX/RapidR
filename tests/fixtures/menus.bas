' QMAINMENU / QPOPUPMENU / QMENUITEM from the shared model: CREATE nesting
' and AddItems, a separator, ShortCut, radio items (checking one unchecks
' the others), Count and MenuIndex, a pop-up menu as a PopupMenu.
CREATE Form AS QFORM
  Caption = "menus"
  CREATE MainMenu AS QMAINMENU
    CREATE FileMenu AS QMENUITEM
      Caption = "&File"
      CREATE NewItem AS QMENUITEM
        Caption = "&New"
        ShortCut = "Ctrl+N"
        OnClick = NewClick
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
        OnClick = ExpertClick
      END CREATE
    END CREATE
  END CREATE
  CREATE Pop AS QPOPUPMENU
    CREATE P1 AS QMENUITEM
      Caption = "One"
    END CREATE
  END CREATE
  CREATE Lbl AS QLABEL
    Width = 300
    Caption = "start"
    PopupMenu = Pop
  END CREATE
END CREATE
DIM Quit AS QMENUITEM
Quit.Caption = "E&xit"
Quit.Enabled = 0
FileMenu.AddItems Quit
Form.ShowModal

SUB ExpertClick
  Expert.Checked = 1
END SUB

SUB NewClick
  Lbl.Caption = "new" + STR$(Beg.Checked) + STR$(Expert.Checked) + STR$(FileMenu.Count) + STR$(Quit.MenuIndex) + NewItem.ShortCut
END SUB
