' RTOOLBAR (RapidR Studio's toolbar, docs/ide-components.md 3.8): icon
' buttons with RapidR's icons, separators, a toggle, a disabled button and
' a QCOOLBTN placed on it (after its buttons); a narrow customizable bar
' whose last button goes behind its "»" button; a bar of bigger buttons
' with captions. The user clicks Save, the disabled Stop (nothing), the
' Grid toggle, the cool button, the bar itself; on the narrow bar the
' "»" button and Redo in its menu, then the menu's "Cut" line (hidden:
' Layout keeps it); the mouse rests on Open.
DIM log AS STRING

SUB ButtonClicked (Name AS STRING, Command AS STRING)
  log = log + " " + Name + "/" + Command
END SUB

SUB BarClicked
  log = log + " click:" + Bar.ClickedButton
END SUB

SUB CoolClicked
  log = log + " cool"
END SUB

SUB Report
  lbl.Caption = log
  info.Caption = STR$(Bar.ButtonCount) + " " + Bar.Button(3) + " " + STR$(Bar.ButtonDown("grid")) + " " + STR$(Bar.ButtonEnabled("stop")) + " [" + Bar2.Layout + "] " + Bar.ButtonHint("open") + STR$(Cool.Left)
END SUB

CREATE Form AS QFORM
  Caption = "Toolbars"
  Width = 520: Height = 260
  CREATE Bar AS RTOOLBAR
    Align = 1 ' alTop
    OnButtonClick = ButtonClicked
    OnClick = BarClicked
    CREATE Cool AS QCOOLBTN
      Left = 4: Top = 4: Width = 60: Height = 24
      Caption = "Cool"
      OnClick = CoolClicked
    END CREATE
  END CREATE
  CREATE Bar2 AS RTOOLBAR
    Left = 8: Top = 60: Width = 150: Height = 32
    Customizable = 1
    OnButtonClick = ButtonClicked
  END CREATE
  CREATE Bar3 AS RTOOLBAR
    Left = 8: Top = 104: Width = 488: Height = 40
    ButtonSize = 36: ShowCaptions = 1
  END CREATE
  CREATE BReport AS QBUTTON
    Left = 8: Top = 156: Caption = "Report"
    OnClick = Report
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 8: Top = 188: Width = 496
  END CREATE
  CREATE Info AS QLABEL
    Left = 8: Top = 208: Width = 496
  END CREATE
END CREATE

Bar.AddButton "new", "file.new", "New file", "file.new"
Bar.AddButton "open", "open", "Open a file", "file.open"
Bar.AddButton "save", "save", "Save", "file.save"
Bar.AddSeparator
Bar.AddButton "run", "run", "Run", "run.start"
Bar.AddButton "stop", "stop", "Stop", "run.stop"
Bar.ButtonEnabled "stop", 0
Bar.AddSeparator
Bar.AddToggle "grid", "designer.showGrid", "Show the grid", "designer.showGrid"

Bar2.AddButton "cut", "edit.cut", "Cut", "edit.cut"
Bar2.AddButton "copy", "edit.copy", "Copy", "edit.copy"
Bar2.AddButton "paste", "edit.paste", "Paste", "edit.paste"
Bar2.AddButton "undo", "edit.undo", "Undo", "edit.undo"
Bar2.AddButton "redo", "edit.redo", "Redo", "edit.redo"

Bar3.AddButton "build", "run.build", "Build the project", "run.build", "Build"
Bar3.AddButton "run", "run", "Run the program", "run.start", "Run"
Bar3.AddToggle "debug", "debug.start", "Debug", "debug.start", "Debug"
Bar3.ButtonDown "debug", 1
Bar3.AddSeparator
Bar3.AddButton "find", "edit.find", "Find", "edit.find"
Form.ShowModal
