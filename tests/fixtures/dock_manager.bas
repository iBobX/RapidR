' RDOCKMANAGER (RapidR Studio's docking, docs/ide-components.md 3.7): an
' IDE's layout — Explorer left, Properties right, Output and Problems
' tabbed below the documents, the Toolbox auto-hidden on the left, two
' documents in the MDI area. The user clicks a tab, drags a splitter,
' slides the Toolbox out, drags Properties onto the compass (below
' Explorer), moves Output with the keyboard (Ctrl+Shift+M's MovePane, the
' arrows, Enter), F6; the program saves the layout, changes it and loads
' it back (the same text), switches to tabbed documents and closes them
' (OnDocumentClose's Cancel keeps one). Pane names come back lowercase.
DIM log AS STRING
DIM saved AS STRING

SUB PaneChanged (Name AS STRING)
  log = log + " p:" + Name
END SUB

SUB DocActive (Name AS STRING)
  log = log + " a:" + Name
END SUB

SUB DocClosing (Name AS STRING, Cancel AS INTEGER)
  log = log + " c:" + Name
  IF Name = "doc1" THEN Cancel = 1
END SUB

SUB LayoutChanged
  log = log + " L"
END SUB

SUB RoundTrip
  saved = Dock.SaveLayout
  Dock.HidePane("Explorer")
  Dock.AutoHide("Props", 1)
  log = log + " [" + Dock.PaneState("Explorer") + " " + Dock.PaneState("Props") + "]"
  IF Dock.LoadLayout(saved) THEN log = log + " loaded"
  IF Dock.SaveLayout = saved THEN log = log + " same" ELSE log = log + " DIFFERENT"
END SUB

SUB MoveOutput
  Dock.MovePane("Output")
END SUB

SUB Tabbed
  Dock.DocumentMode = "tabs"
  Dock.ClosePane("Doc1")
  Dock.ClosePane("Doc2")
END SUB

SUB Report
  lbl.Caption = log
  info.Caption = Dock.DocumentMode + " " + STR$(Dock.PaneCount) + " " + Dock.Pane(0) + " " + Dock.ActivePane + " " + Dock.ActiveDocument + " | " + Dock.PaneState("Explorer") + " " + Dock.PaneState("Props") + " " + Dock.PaneState("Output") + " " + Dock.PaneState("Problems") + " " + Dock.PaneState("Toolbox") + " " + Dock.PaneState("Doc1") + " | " + STR$(Explorer.Width) + "x" + STR$(Explorer.Height) + " " + STR$(Props.Width) + "x" + STR$(Props.Height)
END SUB

CREATE Form AS QFORM
  Caption = "RapidR Studio docking"
  Width = 900: Height = 640
  CREATE Dock AS RDOCKMANAGER
    Left = 0: Top = 0: Width = 884: Height = 520
    OnPaneChange = PaneChanged
    OnDocumentActivate = DocActive
    OnDocumentClose = DocClosing
    OnLayoutChange = LayoutChanged
  END CREATE
  CREATE Explorer AS QLISTBOX
  END CREATE
  CREATE Props AS QLISTBOX
  END CREATE
  CREATE Output AS QRICHEDIT
    Text = "Build started" + CHR$(13) + CHR$(10) + "0 errors, 0 warnings"
  END CREATE
  CREATE Problems AS QLISTBOX
  END CREATE
  CREATE Toolbox AS QLISTBOX
  END CREATE
  CREATE Doc1 AS QRICHEDIT
    Text = "CREATE Form1 AS QFORM"
  END CREATE
  CREATE Doc2 AS QRICHEDIT
    Text = "SUB Main"
  END CREATE
  CREATE bSave AS QBUTTON
    Caption = "Save": Left = 0: Top = 530: OnClick = RoundTrip
  END CREATE
  CREATE bMove AS QBUTTON
    Caption = "Move": Left = 80: Top = 530: OnClick = MoveOutput
  END CREATE
  CREATE bTabs AS QBUTTON
    Caption = "Tabs": Left = 160: Top = 530: OnClick = Tabbed
  END CREATE
  CREATE bReport AS QBUTTON
    Caption = "Report": Left = 240: Top = 530: OnClick = Report
  END CREATE
  CREATE lbl AS QLABEL
    Left = 0: Top = 560: Width = 884
  END CREATE
  CREATE info AS QLABEL
    Left = 0: Top = 580: Width = 884
  END CREATE
END CREATE
Explorer.AddItems "Project1", "Form1.rr", "Module1.rr"
Props.AddItems "Caption", "Width", "Height"
Problems.AddItems "No problems"
Toolbox.AddItems "QBUTTON", "QEDIT", "QLABEL"
Dock.AddPane(Explorer, "Explorer", "left", "explorer")
Dock.AddPane(Props, "Properties", "right", "properties")
Dock.AddPane(Output, "Output", "bottom:documents", "output")
Dock.AddPane(Problems, "Problems", "tab:output", "problems")
Dock.AddPane(Toolbox, "Toolbox", "autohide:left", "toolbox")
Dock.AddPane(Doc1, "Form1.rr", "documents", "form")
Dock.AddPane(Doc2, "Module1.rr", "documents", "code")
log = "-"
Form.ShowModal
