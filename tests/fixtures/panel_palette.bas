' RCOMMANDPALETTE (docs/ide-components.md 3.8): commands found by typing.
' Shown five times: "sa" typed, Down, Enter (OnCommand file.saveall);
' Escape (OnCancel); a click elsewhere takes the focus (OnCancel); a
' click on a row (OnCommand file.save); "s" typed — the ones used last
' first each time, Run: Start disabled.
DIM log AS STRING

SUB Ran (Id AS STRING)
  log = log + " run:" + Id
END SUB

SUB Cancelled
  log = log + " cancel"
END SUB

SUB Open
  Pal.Show
  log = log + " first:" + Pal.Command(0)
END SUB

SUB Report
  lbl.Caption = log
  info.Caption = STR$(Pal.Count) + " " + Pal.Selected + " [" + Pal.Filter + "] " + STR$(Pal.Visible) + " " + STR$(Pal.CommandCount)
END SUB

CREATE Form AS QFORM
  Caption = "Command palette"
  Width = 700: Height = 480
  CREATE Editor AS QRICHEDIT
    Left = 0: Top = 0: Width = 692: Height = 300
    Text = "CREATE Form AS QFORM" + CHR$(13) + CHR$(10) + "  Caption = ""Hello"""
  END CREATE
  CREATE Edit1 AS QEDIT
    Left = 10: Top = 310: Width = 200
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 10: Top = 340: Width = 670: Height = 40
    WordWrap = 1
  END CREATE
  CREATE Info AS QLABEL
    Left = 10: Top = 384: Width = 400
  END CREATE
  CREATE BOpen AS QBUTTON
    Left = 420: Top = 400: Caption = "Commands"
    OnClick = Open
  END CREATE
  CREATE BReport AS QBUTTON
    Left = 510: Top = 400: Caption = "Report"
    OnClick = Report
  END CREATE
  CREATE Pal AS RCOMMANDPALETTE
    OnCommand = Ran
    OnCancel = Cancelled
  END CREATE
END CREATE
Pal.AddCommand("file.save", "Save", "Ctrl+S", "File", "save")
Pal.AddCommand("file.saveall", "Save All", "Ctrl+K S", "File", "save-all")
Pal.AddCommand("run.start", "Start", "F5", "Run", "run")
Pal.AddCommand("view.palette", "Command Palette", "Ctrl+Shift+P", "View", "command-palette")
Pal.AddCommand("help.about", "About RapidR", "", "Help", "info")
Pal.CommandEnabled("run.start", 0)
Form.ShowModal
